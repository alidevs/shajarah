use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method},
    routing::{get, post, put},
    Router,
};
use rand::Rng;
use server::{
    api::{
        members::routes::{add_member, delete_member, edit_member, get_members, get_members_flat},
        sessions::refresh_session,
        users::routes::{login, logout, me},
    },
    pages::{admin_page, login_page},
    AppState, Config, ConfigError, InnerAppState,
};

#[cfg(debug_assertions)]
use server::api::users::routes::create_user;

use sqlx::PgPool;
use tower_cookies::{CookieManagerLayer, Key};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, services::ServeDir};

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let config = match Config::load_config() {
        Ok(config) => config,
        Err(err) => match &err {
            ConfigError::IoError(err) if err.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("GENERATING CONFIG FILE WITH SECRET");

                let mut secret = [0u8; 64];
                rand::thread_rng().fill(&mut secret);

                let secret = String::from_utf8_lossy(&secret).to_string();

                let config = Config {
                    cookie_secret: secret,
                    ..Default::default()
                };

                let config_str =
                    toml::to_string(&config).expect("Serialize config struct to toml string");

                std::fs::write("config.toml", config_str)
                    .expect("writing config toml string to config.toml");

                config
            }
            _ => {
                panic!("{:#?}", err);
            }
        },
    };

    let app_state = AppState {
        inner: Arc::new(InnerAppState {
            db_pool: pool,
            cookies_secret: Key::from(config.cookie_secret.as_bytes()),
        }),
    };

    let app = Router::new()
        .route("/admin", get(admin_page))
        .route("/login", get(login_page))
        .route("/api/members", get(get_members).post(add_member))
        .route("/api/members/:id", put(edit_member).delete(delete_member))
        .route("/api/members/flat", get(get_members_flat))
        .route("/api/users/logout", get(logout))
        .route("/api/users/login", post(login))
        .route("/api/users/me", get(me))
        .nest_service("/assets", ServeDir::new("assets"));

    #[cfg(debug_assertions)]
    let app = app.route("/api/users", post(create_user));

    let app = app
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:3001".parse::<HeaderValue>().unwrap(),
                    "http://localhost:9393".parse::<HeaderValue>().unwrap(),
                    "http://192.168.0.132:3001".parse::<HeaderValue>().unwrap(),
                    "http://192.168.0.132:3030".parse::<HeaderValue>().unwrap(),
                    "https://shajarah.bksalman.com"
                        .parse::<HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([Method::GET]),
        )
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            refresh_session,
        ))
        .layer(CookieManagerLayer::new())
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(25 * 1024 * 1024 /* 25mb */))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();

    log::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
