use zero2prod::startup::run;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let listener = std::net::TcpListener::bind("0.0.0.0:8080").expect("Unable to bind to a socket");
    println!("Bound to {}", listener.local_addr().unwrap());
    run(listener)?.await
}
