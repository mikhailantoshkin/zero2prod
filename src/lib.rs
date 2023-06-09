use axum::{http::StatusCode, routing::get, Router};

type AxumServer =
    hyper::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub fn run(listener: std::net::TcpListener) -> Result<AxumServer, hyper::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}
