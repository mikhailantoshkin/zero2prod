use std::net::{IpAddr, SocketAddr};

use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use hyper::{Body, Request};
use sqlx::{PgPool, Pool, Postgres};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type AxumServer =
    hyper::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{health_check, subscribe},
};

pub struct Application {
    local_addr: SocketAddr,
    server: AxumServer,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, anyhow::Error> {
        let pool = get_connection_pool(&config.database).await?;
        let email_client = EmailClient::new(
            config.email_client.base_url,
            config.email_client.sender_email,
            config.email_client.authorization_token,
            config.email_client.timeout_millis,
        );
        let addr = format!("{}:{}", config.app.host, config.app.port);
        let listener = std::net::TcpListener::bind(addr).context("Unable to bind to a socket")?;
        let local_addr = listener.local_addr()?;
        tracing::info!("Listening on {}", &local_addr);
        let server = build_server(listener, pool, email_client)?;
        Ok(Application {
            local_addr: local_addr,
            server: server,
        })
    }
    pub fn port(&self) -> u16 {
        self.local_addr.port()
    }
    pub fn addr(&self) -> IpAddr {
        self.local_addr.ip()
    }
    pub async fn run_forever(self) -> Result<(), hyper::Error> {
        self.server.await
    }
}

pub async fn get_connection_pool(config: &DatabaseSettings) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPool::connect_with(config.with_db()).await
}

pub fn build_server(
    listener: std::net::TcpListener,
    conn: PgPool,
    email_client: EmailClient,
) -> Result<AxumServer, hyper::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subsriptions", post(subscribe))
        .layer(
            TraceLayer::new_for_http().make_span_with(|_req: &Request<Body>| {
                tracing::debug_span!("http-request", request_id = %Uuid::new_v4())
            }),
        )
        .with_state(conn).with_state(email_client);
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}
