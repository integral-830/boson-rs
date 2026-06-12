use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing;

use crate::handler::handle;
use crate::store::Store;

pub async fn run_server(
    listener: TcpListener,
    store: Arc<Store>,
    conn_semaphore: Arc<Semaphore>,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let mut join_set = JoinSet::new();

    loop {
        tokio::select! {
            biased;
            ()= shutdown.cancelled()=>{
                tracing::info!("shutdown requested");
                break;
            }
            result = listener.accept() =>{
               let (stream, addr) = match result {
                    Ok(val) => val,
                    Err(err) => {
                        tracing::error!("accept error: {err}");
                        continue;
                    },
                };
               tracing::info!(
                    addr = ?addr,
                    "new connection"
                );
               match conn_semaphore.clone().try_acquire_owned() {
                   Ok(permit) => {
                        let store = Arc::clone(&store);
                        let shutdown = shutdown.clone();
                        join_set.spawn(async move {
                            handle(stream, store, permit, shutdown).await;
                        });
                   }
                   Err(tokio::sync::TryAcquireError::NoPermits) => {
                        reject_client(stream).await;
                   }
                   Err(tokio::sync::TryAcquireError::Closed) => {
                        break;
                   }
               }
            }
        }
    }
    tracing::info!("waiting for active connections");
    join_set.join_all().await;
    tracing::info!("all connections closed, exiting");
    Ok(())
}

async fn reject_client(mut stream: TcpStream) {
    let _ = stream.write_all(b"-ERR max clients reached\r\n").await;
}
