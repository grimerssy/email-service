use super::IdempotencyKey;
use crate::DbPool;
use actix_web::{body::to_bytes, http::StatusCode, HttpResponse};
use anyhow::anyhow;
use sqlx::postgres::PgHasArrayType;
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

pub async fn store_response(
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    response: HttpResponse,
    pool: &DbPool,
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
        insert into idempotency(
          user_id,
          idempotency_key,
          response_status_code,
          response_headers,
          response_body,
          created_at
        )
        values ($1, $2, $3, $4, $5, now());
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(pool)
    .await?;
    let response = head.set_body(body).map_into_boxed_body();
    Ok(response)
}

pub async fn get_saved_response(
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    pool: &DbPool,
) -> anyhow::Result<Option<HttpResponse>> {
    let r = match sqlx::query!(
        r#"
        select
          response_status_code,
          response_headers as "response_headers: Vec<HeaderPair>",
          response_body
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
