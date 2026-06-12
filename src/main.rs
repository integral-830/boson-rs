use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::info;

use boson_rs::{server::run_server, store::Store};

const MAX_CONNECTIONS: usize = 1000;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("0.0.0.0:6380").await?;

    info!("listening on 0.0.0.0:6380");

    let store = Arc::new(Store::new());
    let conn_semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let shutdown_int = shutdown.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();

        tracing::info!("SIGINT received");

        shutdown_int.cancel();
    });
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let shutdown_term = shutdown.clone();

        tokio::spawn(async move {
            let mut sigterm = signal(SignalKind::terminate()).unwrap();

            sigterm.recv().await;

            tracing::info!("SIGTERM received");

            shutdown_term.cancel();
        });
    }
    run_server(listener, store, conn_semaphore, shutdown_clone).await?;

    Ok(())
}
