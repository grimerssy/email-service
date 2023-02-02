use crate::{routes::*, Config, DbPool, EmailClient};
use actix_session::{storage::RedisSessionStore, SessionMiddleware};
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
    pub async fn build(config: Config) -> anyhow::Result<Self> {
        let db_pool = DbPool::connect_lazy_with(config.database.with_db());
        let email_client = EmailClient::new(config.email_client);
        let addr =
            format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(addr)?;
        let port = listener.local_addr().unwrap().port();
        let base_url = AppBaseUrl(config.application.base_url);
        let server = Self::http_server(
            listener,
            db_pool,
            email_client,
            base_url,
            config.application.redis_url,
            config.application.hmac_secret,
        )
        .await?;
        Ok(Self { port, server })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.server.await.map_err(anyhow::Error::from)
    }

    async fn http_server(
        listener: TcpListener,
        db_pool: DbPool,
        email_client: EmailClient,
        base_url: AppBaseUrl,
        redis_url: Secret<String>,
        hmac_secret: Secret<String>,
    ) -> anyhow::Result<ActixServer> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        let base_url = Data::new(base_url);
        let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
        let redis_store =
            RedisSessionStore::new(redis_url.expose_secret()).await?;
        let message_store =
            CookieMessageStore::builder(secret_key.clone()).build();
        let message_framework =
            FlashMessagesFramework::builder(message_store).build();
        HttpServer::new(move || {
            App::new()
                .wrap(message_framework.clone())
                .wrap(SessionMiddleware::new(
                    redis_store.clone(),
                    secret_key.clone(),
                ))
                .wrap(TracingLogger::default())
                .route("/", get().to(home))
                .route("/health_check", get().to(health_check))
                .route("/login", get().to(login_form))
                .route("/login", post().to(login))
                .route("/logout", post().to(logout))
                .route("/admin/dashboard", get().to(admin_dashboard))
                .route("/admin/password", post().to(change_password))
                .route("/admin/password", get().to(change_password_form))
                .route("/subscriptions", post().to(subscribe))
                .route("/subscriptions/confirm", get().to(confirm_subscription))
                .route("/newsletters", post().to(publish_newsletter))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
                .app_data(base_url.clone())
        })
        .listen(listener)
        .map(|s| s.run())
        .map_err(anyhow::Error::from)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
