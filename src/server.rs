use crate::fetch::FetchClient;
use axum::routing::get;
use axum::Router;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

mod index;

struct AppState {
    client: FetchClient,
}

pub async fn run(addr: SocketAddr) -> Result<(), io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("listening on {}", addr);

    let client = Client::builder(TokioExecutor::new()).build(
        HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    );

    let state = Arc::new(AppState { client });

    let app = Router::new()
        .route("/", get(index::get))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new().br(true)),
        );

    axum::serve(listener, app).await?;

    Ok(())
}
