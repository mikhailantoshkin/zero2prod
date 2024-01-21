use std::fmt::{Debug, Display};

use anyhow::Context;

use tokio::task::JoinError;
use zero2prod::configuration::get_config;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber)?;
    let config = get_config().context("Failed to read configuration")?;
    let app = Application::build(config.clone()).await?;
    tracing::info!("Listening on {}:{}", app.addr(), app.port());
    let server = tokio::spawn(app.run_forever());
    let worker = tokio::spawn(run_worker_until_stopped(config));
    tokio::select! {
        o = server => report_exit("API", o),
        o = worker => report_exit("Worker", o),
    }
    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }   
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "'{}' task failed to complete",
                task_name
            )
        }
    }
}
