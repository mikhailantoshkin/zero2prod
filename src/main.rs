use anyhow::Context;
use sqlx::PgPool;
use zero2prod::{configuration::get_config, startup::run};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = get_config().context("Failed to read configuration")?;
    let pool = PgPool::connect(&config.database.connfection_string()).await?;
    let addr = format!("0.0.0.0:{}", config.bind_port);
    let listener = std::net::TcpListener::bind(addr).context("Unable to bind to a socket")?;
    println!("Listening on {}", listener.local_addr()?);
    run(listener, pool)?.await?;
    Ok(())
}
