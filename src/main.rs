use std::fmt::{Debug, Display};

use dotenvy::dotenv;
use tokio::task::JoinError;
use zero2prod::{issue_delivery, telemetry, Config, Server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    telemetry::init("zero2prod", "info", std::io::stdout)
        .expect("Failed to initialize telemetry");
    let config = Config::init().expect("Failed to initialize config");
    let server = tokio::spawn(Server::build(config.clone()).await?.run());
    let worker = tokio::spawn(issue_delivery::run_worker(config));
    tokio::select! {
        o = server => {report_exit("Server", o)},
        o = worker => {report_exit("Background worker", o)}
    };
    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, JoinError>,
) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{task_name} has exited"),
        Ok(Err(e)) => tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{task_name} failed",
        ),
        Err(e) => tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{task_name} failed to complete"
        ),
    }
}
