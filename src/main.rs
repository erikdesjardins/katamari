use std::io;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod err;
mod fetch;
mod opt;
mod server;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), io::Error> {
    let opt::Options {
        verbose,
        listen_addr,
    } = clap::Parser::parse();

    tracing_subscriber::registry()
        .with(match verbose {
            0 => LevelFilter::INFO,
            1 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        })
        .with(tracing_subscriber::fmt::layer())
        .init();

    server::run(listen_addr).await?;

    Ok(())
}
