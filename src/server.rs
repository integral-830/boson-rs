use std::sync::Arc;

use tokio::net::TcpListener;
use tracing;

use crate::handler::handle;
use crate::store::Store;

pub async fn run_server(listener: TcpListener, store: Arc<Store>) -> std::io::Result<()> {
    loop {
        let (stream, addr) = listener.accept().await?;

        tracing::info!(
            addr = ?addr,
            "new connection"
        );

        let store = Arc::clone(&store);

        tokio::spawn(async move {
            handle(stream, store).await;
        });
    }
}
