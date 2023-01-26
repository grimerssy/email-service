use crate::{routes::*, Config, DbPool, EmailClient};
use actix_web::{
    dev::Server as ActixServer,
    web::{get, post, Data},
    App, HttpServer,
};
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
        let addr = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(addr)?;
        let port = listener.local_addr().unwrap().port();
        let base_url = AppBaseUrl(config.application.base_url);
        let server = Self::http_server(listener, db_pool, email_client, base_url)?;
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
    ) -> std::io::Result<ActixServer> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        let base_url = Data::new(base_url);
        HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
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
