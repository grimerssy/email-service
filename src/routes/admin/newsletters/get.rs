use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use uuid::Uuid;

pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let idempotency_key = Uuid::new_v4();
    let mut msgs = String::new();
    for m in flash_messages.iter() {
        writeln!(msgs, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Publish Newsletter Issue</title>
            </head>
            <body>
                {msgs}
                <form action="/admin/newsletters" method="post">
                    <label>Title:<br>
                        <input
                            type="text"
                            placeholder="Enter the issue title"
                            name="title"
                        >
                    </label>
                    <br>
                    <label>Plain text content:<br>
                        <textarea
                            placeholder="Enter the content in plain text"
                            name="textContent"
                            rows="20"
                            cols="50"
                        ></textarea>
                    </label>
                    <br>
                    <label>HTML content:<br>
                        <textarea
                            placeholder="Enter the content in HTML format"
                            name="htmlContent"
                            rows="20"
                            cols="50"
                        ></textarea>
                    </label>
                    <br>
                    <input hidden type="text" name="idempotencyKey" value="{idempotency_key}">
                    <button type="submit">Publish</button>
                </form>
                <p><a href="/admin/dashboard">&lt;- Back</a></p>
            </body>
            </html>
            "#,
        )))
}
