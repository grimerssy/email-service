use std::time::Duration;

use crate::{domain::SubscriberEmail, Config, Database, DbPool, EmailClient};
use sqlx::Transaction;
use tracing::{field::display, Span};
use uuid::Uuid;

pub async fn run_worker(config: Config) -> anyhow::Result<()> {
    let pool = DbPool::connect_lazy_with(config.database.with_db());
    let email_client = EmailClient::new(config.email_client);
    worker_loop(&pool, &email_client).await
}

async fn worker_loop(
    pool: &DbPool,
    email_client: &EmailClient,
) -> anyhow::Result<()> {
    loop {
        match try_execute_task(pool, email_client).await {
            Ok(ExecutionOutcome::Completed) => {}
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

pub enum ExecutionOutcome {
    Completed,
    EmptyQueue,
}

#[tracing::instrument(skip_all, err, fields(
    newsletter_issue_id=tracing::field::Empty,
    subscriber_email=tracing::field::Empty
))]
pub async fn try_execute_task(
    pool: &DbPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionOutcome> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (transaction, issue_id, email) = task.unwrap();
    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));
    let email = match SubscriberEmail::try_from(email) {
        Ok(email) => email,
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. \
                 Their stored contact details are invalid",
            );
            return Ok(ExecutionOutcome::Completed);
        }
    };
    let NewsletterIssue {
        title,
        text_content,
        html_content,
    } = get_issue(&issue_id, pool).await?;
    if let Err(e) = email_client
        .send_email(&email, &title, &text_content, &html_content)
        .await
    {
        tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "Failed to deliver issue to a \
             confirmed subscriber. Skipping",
        );
    }
    delete_task(&issue_id, email.as_ref(), transaction).await?;
    Ok(ExecutionOutcome::Completed)
}

#[derive(Debug, Clone)]
struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_issue(
    issue_id: &Uuid,
    pool: &DbPool,
) -> sqlx::Result<NewsletterIssue> {
    sqlx::query_as!(
        NewsletterIssue,
        r#"
        select title, text_content, html_content
        from newsletter_issues
        where newsletter_issue_id = $1;
        "#,
        issue_id
    )
    .fetch_one(pool)
    .await
}

async fn dequeue_task(
    pool: &DbPool,
) -> anyhow::Result<Option<(Transaction<'static, Database>, Uuid, String)>> {
    let mut transaction = pool.begin().await?;
    let row = sqlx::query!(
        r#"
        select newsletter_issue_id, subscriber_email
        from issue_delivery_queue
        for update
        skip locked
        limit 1;
        "#
    )
    .fetch_optional(&mut transaction)
    .await?
    .map(|r| (transaction, r.newsletter_issue_id, r.subscriber_email));
    Ok(row)
}

async fn delete_task(
    issue_id: &Uuid,
    email: &str,
    mut transaction: Transaction<'_, Database>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        delete from issue_delivery_queue
        where newsletter_issue_id = $1
        and subscriber_email = $2;
        "#,
        issue_id,
        email,
    )
    .execute(&mut transaction)
    .await?;
    transaction.commit().await.map(|_| ())
}
