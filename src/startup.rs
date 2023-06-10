use axum::{
    routing::{get, post},
    Router,
};

type AxumServer =
    hyper::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

use crate::routes::{health_check, subscriptions};

pub fn run(listener: std::net::TcpListener) -> Result<AxumServer, hyper::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subsriptions", post(subscriptions));
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}
