use axum::routing::get;
use axum::Router;
use std::io;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

pub async fn run(addr: SocketAddr) -> Result<(), io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("listening on {}", addr);

    let app = Router::new()
        .route("/", get(index))
        .layer(TraceLayer::new_for_http());

    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> &'static str {
    "Hello, World!"
}
