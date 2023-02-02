use dotenvy::dotenv;
use zero2prod::{telemetry, Config, Server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    telemetry::init("zero2prod", "info", std::io::stdout)
        .expect("Failed to initialize telemetry");
    let config = Config::init().expect("Failed to initialize config");
    Server::build(config).await?.run().await.map(|_| ())
}
