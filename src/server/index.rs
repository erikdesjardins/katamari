use crate::server::AppState;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use http_body_util::BodyExt;
use std::sync::Arc;

pub async fn get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let output = state
        .client
        .get("https://erikdesjardins.io/".parse().unwrap())
        .await
        .unwrap();

    Html(output.into_body().collect().await.unwrap().to_bytes())
}
