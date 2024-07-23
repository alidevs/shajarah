use std::sync::Arc;

use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router,
};
use server::{
    api::members::{add_member, get_members},
    AppState,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let app_state = Arc::new(AppState { db_pool: pool });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/api/members", get(get_members).post(add_member))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:3001".parse::<HeaderValue>().unwrap(),
                    "http://localhost:9393".parse::<HeaderValue>().unwrap(),
                    "https://shajarah.bksalman.com"
                        .parse::<HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([Method::GET]),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8383").await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
