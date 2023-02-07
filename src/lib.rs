mod auth;
pub mod configuration;
mod domain;
mod email_client;
mod idempotency;
pub mod issue_delivery;
mod routes;
mod server;
mod session;
pub mod telemetry;
mod utils;

pub use configuration::Config;
pub use email_client::EmailClient;
pub use server::Server;
pub use session::Session;

pub type Database = sqlx::Postgres;
pub type DbPool = sqlx::Pool<Database>;
