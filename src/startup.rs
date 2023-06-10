use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

type AxumServer =
    hyper::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

use crate::routes::{health_check, subscriptions};

pub fn run(listener: std::net::TcpListener, conn: PgPool) -> Result<AxumServer, hyper::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subsriptions", post(subscriptions))
        .with_state(conn);
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}
