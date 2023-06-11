use anyhow::Context;

use sqlx::PgPool;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::{configuration::get_config, startup::run};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber)?;
    let config = get_config().context("Failed to read configuration")?;
    let pool = PgPool::connect_with(config.database.with_db()).await?;
    let addr = format!("{}:{}", config.app.host, config.app.port);
    let listener = std::net::TcpListener::bind(addr).context("Unable to bind to a socket")?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    run(listener, pool)?.await?;
    Ok(())
}
