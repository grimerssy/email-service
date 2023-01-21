use crate::{
    routes::{health_check, subscribe},
    Config, DbPool, EmailClient,
};
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

impl Server {
    pub fn build(config: Config) -> std::io::Result<Self> {
        let db_pool = DbPool::connect_lazy_with(config.database.with_db());
        let email_client = EmailClient::new(config.email_client);
        let addr = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(addr)?;
        let port = listener.local_addr().unwrap().port();
        let server = Self::http_server(listener, db_pool, email_client)?;
        Ok(Self { port, server })
    }

    pub async fn run(self) -> std::io::Result<()> {
        self.server.await
    }

    fn http_server(
        listener: TcpListener,
        db_pool: DbPool,
        email_client: EmailClient,
    ) -> std::io::Result<ActixServer> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .route("/health_check", get().to(health_check))
                .route("/subscriptions", post().to(subscribe))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
        })
        .listen(listener)
        .map(|s| s.run())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
