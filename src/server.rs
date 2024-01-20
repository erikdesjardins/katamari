use axum::routing::get;
use axum::Router;
use std::io;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

mod index;

pub async fn run(addr: SocketAddr) -> Result<(), io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("listening on {}", addr);

    let app = Router::new().route("/", get(index::get)).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new().br(true)),
    );

    axum::serve(listener, app).await?;

    Ok(())
}
