use anyhow::Context;
use secrecy::ExposeSecret;
use sqlx::PgPool;

use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::{configuration::get_config, startup::run};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber)?;
    let config = get_config().context("Failed to read configuration")?;
    let pool = PgPool::connect(config.database.connfection_string().expose_secret()).await?;
    let addr = format!("0.0.0.0:{}", config.bind_port);
    let listener = std::net::TcpListener::bind(addr).context("Unable to bind to a socket")?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    run(listener, pool)?.await?;
    Ok(())
}
