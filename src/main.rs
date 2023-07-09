use anyhow::Context;

use zero2prod::configuration::get_config;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber)?;
    let config = get_config().context("Failed to read configuration")?;
    let app = Application::build(config).await?;
    tracing::info!("Listening on {}:{}", app.addr(), app.port());
    app.run_forever().await?;
    Ok(())
}
