use std::{env, sync::Arc};

use tokio::net::TcpListener;

use tracing::info;

use boson_rs::{handler::handle, store::Store};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(6380);

    let addr = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&addr).await?;

    let store = Arc::new(Store::new());

    info!("listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

        info!(
            addr = ?addr,
            "new connection"
        );

        let store = Arc::clone(&store);

        tokio::spawn(async move {
            handle(stream, store).await;
        });
    }
}
