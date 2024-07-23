use std::sync::Arc;

use axum::{routing::get, Router};
use server::{api::members::get_members, AppState, Node};

#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState {
        root_member: Node::new(
            1,
            String::from("سلمان"),
            vec![
                Node::new(
                    2,
                    String::from("سلمان"),
                    vec![
                        Node::new(6, String::from("سلمان"), vec![]),
                        Node::new(7, String::from("سلمان"), vec![]),
                        Node::new(8, String::from("سلمان"), vec![]),
                    ],
                ),
                Node::new(3, String::from("سلمان"), vec![]),
                Node::new(
                    4,
                    String::from("سلمان"),
                    vec![
                        Node::new(
                            9,
                            String::from("سلمان"),
                            vec![
                                Node::new(11, String::from("سلمان"), vec![]),
                                Node::new(12, String::from("سلمان"), vec![]),
                            ],
                        ),
                        Node::new(10, String::from("سلمان"), vec![]),
                    ],
                ),
                Node::new(5, String::from("سلمان"), vec![]),
            ],
        ),
    });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/members", get(get_members))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8383").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
