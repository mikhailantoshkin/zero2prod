use std::{
    net::{IpAddr, SocketAddr},
    ops::Deref,
};

use anyhow::Context;
use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{FromRef, MatchedPath},
    http::StatusCode,
    middleware,
    routing::{get, post},
    BoxError, Router,
};
use axum_extra::extract::cookie::Key;
use axum_login::AuthManagerLayerBuilder;
use fred::{
    clients::RedisClient,
    interfaces::ClientLike,
    types::{ConnectHandle, RedisConfig},
};
use hyper::Request;
use secrecy::{ExposeSecret, Secret};
use sqlx::{PgPool, Pool, Postgres};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tower_sessions::{cookie::time::Duration, Expiry, RedisStore, SessionManagerLayer};
use uuid::Uuid;

use crate::{
    authentication::middleware::{auth_middleware, Backend},
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        admin_dashboard, change_passord_form, change_password, health_check, home, log_out, login,
        login_form, publish_newsletter, publish_newsletter_form, subscribe, subscription_confirm,
    },
};

pub struct Application {
    local_addr: SocketAddr,
    server: Server,
    redis_handle: ConnectHandle,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, anyhow::Error> {
        let pool = get_connection_pool(&config.database).await?;
        let email_client = config.email_client.client();
        let addr = format!("{}:{}", config.app.host, config.app.port);
        let listener = TcpListener::bind(addr)
            .await
            .context("Unable to bind to a socket")?;
        let local_addr = listener.local_addr()?;
        let redis_config = RedisConfig::from_url(config.redis_uri.expose_secret())
            .expect("Failed to create redis settings");
        let redis_client = RedisClient::new(redis_config, None, None, None);
        let redis_handle = redis_client.connect();
        redis_client.wait_for_connect().await?;
        tracing::info!("Listening on {}", &local_addr);
        let server = build_server(
            listener,
            pool,
            email_client,
            config.app.base_url,
            &config.app.hmac_secret,
            redis_client,
        )?;
        Ok(Application {
            local_addr,
            server,
            redis_handle,
        })
    }

    pub fn port(&self) -> u16 {
        self.local_addr.port()
    }

    pub fn addr(&self) -> IpAddr {
        self.local_addr.ip()
    }

    pub async fn run_forever(self) -> Result<(), anyhow::Error> {
        self.server.serve().await?;
        self.redis_handle.await??;
        Ok(())
    }
}

pub async fn get_connection_pool(config: &DatabaseSettings) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPool::connect_with(config.with_db()).await
}

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

impl Deref for HmacSecret {
    type Target = Secret<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct Server {
    listener: TcpListener,
    app: Router,
}
impl Server {
    pub fn new(listener: TcpListener, app: Router) -> Self {
        Self { listener, app }
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        axum::serve(self.listener, self.app).await
    }
}

#[derive(FromRef, Clone)]
struct AppState {
    conn: PgPool,
    email_client: EmailClient,
    base_url: ApplicationBaseUrl,
    flash_config: axum_flash::Config,
}

fn build_server(
    listener: TcpListener,
    conn: PgPool,
    email_client: EmailClient,
    base_url: String,
    secret: &[u8],
    redis_client: RedisClient,
) -> Result<Server, hyper::Error> {
    let session_store = RedisStore::new(redis_client);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(10)));

    let backend = Backend::new(conn.clone());

    let auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(AuthManagerLayerBuilder::new(backend, session_layer).build());

    let admin_router = Router::new()
        .route("/admin/dashboard", get(admin_dashboard))
        .route(
            "/admin/password",
            get(change_passord_form).post(change_password),
        )
        .route("/admin/logout", post(log_out))
        .route(
            "/admin/newsletter",
            get(publish_newsletter_form).post(publish_newsletter),
        );

    let tracig_layer = TraceLayer::new_for_http().make_span_with(|req: &Request<Body>| {
                let method = req.method();
                let uri = req.uri();
                let matched_path = req.extensions().get::<MatchedPath>().map(|p| p.as_str());

                tracing::debug_span!("http-request", %method, %uri, matched_path, request_id = %Uuid::new_v4())
            });

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(subscription_confirm))
        .route("/home", get(home))
        .route("/login", get(login_form).post(login))
        .merge(admin_router.route_layer(middleware::from_fn(auth_middleware)))
        .layer(auth_service)
        .layer(tracig_layer)
        .with_state(AppState {
            conn,
            email_client,
            base_url: ApplicationBaseUrl(base_url),
            flash_config: axum_flash::Config::new(Key::derive_from(secret)),
        });
    Ok(Server::new(listener, app))
}
