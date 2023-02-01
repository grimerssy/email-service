use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_msg = String::new();
    flash_messages
        .iter()
        .filter(|m| m.level() == Level::Error)
        .for_each(|m| {
            writeln!(error_msg, "<p><i>{}</i></p>", m.content()).unwrap()
        });
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
              <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8" />
                <title>Login</title>
              </head>
                {error_msg}
              <body>
                <form action="/login" method="post">
                  <label>
                    Username
                    <input type="text" placeholder="Enter Username" name="username" />
                  </label>
                  <label>
                    Password
                    <input type="password" placeholder="Enter Password" name="password" />
                  </label>
                  <button type="submit">Login</button>
                </form>
              </body>
            </html>
            "#
        ))
}
