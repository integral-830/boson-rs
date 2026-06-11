use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

use boson_rs::{server::run_server, store::Store};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("0.0.0.0:6380").await?;

    info!("listening on 0.0.0.0:6380");

    let store = Arc::new(Store::new());

    run_server(listener, store).await?;

    Ok(())
}
