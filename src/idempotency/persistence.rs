use super::IdempotencyKey;
use crate::{Database, DbPool};
use actix_web::{body::to_bytes, http::StatusCode, HttpResponse};
use anyhow::anyhow;
use sqlx::{postgres::PgHasArrayType, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPair {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPair {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

#[derive(Debug)]
pub enum NextAction {
    StartProcessing(Transaction<'static, Database>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_process(
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    pool: &DbPool,
) -> anyhow::Result<NextAction> {
    let mut transaction = pool.begin().await?;
    match sqlx::query!(
        r#"
        insert into idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        values ($1, $2, now())
        on conflict do nothing;
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut transaction)
    .await?
    .rows_affected()
    {
        0 => get_saved_response(user_id, idempotency_key, pool)
            .await?
            .ok_or_else(|| anyhow!("Failed to get saved response"))
            .map(NextAction::ReturnSavedResponse),
        1 => Ok(NextAction::StartProcessing(transaction)),
        _ => unreachable!(),
    }
}

pub async fn store_response(
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    response: HttpResponse,
    mut transaction: Transaction<'_, Database>,
) -> anyhow::Result<HttpResponse> {
    let (head, body) = response.into_parts();
    let status_code = head.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(head.headers().len());
        for (name, value) in head.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPair { name, value });
        }
        h
    };
    let body = to_bytes(body).await.map_err(|e| anyhow!("{e}"))?;
    sqlx::query_unchecked!(
        r#"
        update idempotency
        set
            response_status_code = $1,
            response_headers = $2,
            response_body = $3
        where user_id = $4
        and idempotency_key = $5;
        "#,
        status_code,
        headers,
        body.as_ref(),
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(&mut transaction)
    .await?;
    transaction.commit().await?;
    let response = head.set_body(body).map_into_boxed_body();
    Ok(response)
}

async fn get_saved_response(
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    pool: &DbPool,
) -> anyhow::Result<Option<HttpResponse>> {
    let r = match sqlx::query!(
        r#"
        select
          response_status_code as "response_status_code!",
          response_headers as "response_headers!: Vec<HeaderPair>",
          response_body as "response_body!"
        from idempotency
        where user_id = $1
        and idempotency_key = $2;
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?
    {
        Some(r) => r,
        None => return Ok(None),
    };
    let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
    let mut response = HttpResponse::build(status_code);
    for HeaderPair { name, value } in r.response_headers {
        response.insert_header((name, value));
    }
    Ok(Some(response.body(r.response_body)))
}
