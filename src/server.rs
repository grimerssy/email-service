use crate::{routes::*, Config, DbPool, EmailClient};
use actix_web::{
    cookie::Key,
    dev::Server as ActixServer,
    web::{get, post, Data},
    App, HttpServer,
};
use actix_web_flash_messages::{
    storage::CookieMessageStore, FlashMessagesFramework,
};
use secrecy::{ExposeSecret, Secret};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Server {
    port: u16,
    server: ActixServer,
}

#[derive(Clone, Debug)]
pub struct AppBaseUrl(String);

impl AsRef<str> for AppBaseUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Server {
    pub fn build(config: Config) -> std::io::Result<Self> {
        let db_pool = DbPool::connect_lazy_with(config.database.with_db());
        let email_client = EmailClient::new(config.email_client);
        let addr =
            format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(addr)?;
        let port = listener.local_addr().unwrap().port();
        let base_url = AppBaseUrl(config.application.base_url);
        let hmac_secret = config.application.hmac_secret;
        let server = Self::http_server(
            listener,
            db_pool,
            email_client,
            base_url,
            hmac_secret,
        )?;
        Ok(Self { port, server })
    }

    pub async fn run(self) -> std::io::Result<()> {
        self.server.await
    }

    fn http_server(
        listener: TcpListener,
        db_pool: DbPool,
        email_client: EmailClient,
        base_url: AppBaseUrl,
        hmac_secret: Secret<String>,
    ) -> std::io::Result<ActixServer> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        let base_url = Data::new(base_url);
        let message_store = CookieMessageStore::builder(Key::from(
            hmac_secret.expose_secret().as_bytes(),
        ))
        .build();
        let message_framework =
            FlashMessagesFramework::builder(message_store).build();
        HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .wrap(message_framework.clone())
                .route("/", get().to(home))
                .route("/login", get().to(login_form))
                .route("/login", post().to(login))
                .route("/health_check", get().to(health_check))
                .route("/subscriptions", post().to(subscribe))
                .route("/subscriptions/confirm", get().to(confirm_subscription))
                .route("/newsletters", post().to(publish_newsletter))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
                .app_data(base_url.clone())
        })
        .listen(listener)
        .map(|s| s.run())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
