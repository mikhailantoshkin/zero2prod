use axum::{
    routing::{get, post},
    Router,
};
use hyper::{Body, Request};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type AxumServer =
    hyper::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

use crate::routes::{health_check, subscribe};

pub fn run(listener: std::net::TcpListener, conn: PgPool) -> Result<AxumServer, hyper::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subsriptions", post(subscribe))
        .layer(
            TraceLayer::new_for_http().make_span_with(|_req: &Request<Body>| {
                tracing::debug_span!("http-request", request_id = %Uuid::new_v4())
            }),
        )
        .with_state(conn);
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}
