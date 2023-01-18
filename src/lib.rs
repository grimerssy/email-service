pub mod configuration;
mod domain;
mod email_client;
mod routes;
mod startup;
pub mod telemetry;

pub use configuration::Config;
pub use email_client::EmailClient;
pub use startup::run;
