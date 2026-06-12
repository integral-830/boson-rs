use std::sync::Arc;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::exec::execute;
use crate::{
    cmd::parse_command,
    codec::{RespCodec, RespValue},
    store::Store,
};

#[tracing::instrument(skip(stream, store, _permit, shutdown))]
pub async fn handle(
    stream: TcpStream,
    store: Arc<Store>,
    _permit: OwnedSemaphorePermit,
    shutdown: CancellationToken,
) {
    let mut framed = Framed::new(stream, RespCodec);
    loop {
        tokio::select! {
            biased;

            _ = shutdown.cancelled() => {
                tracing::info!("shutdown");
                break;
            }

            result = framed.next() => {
                match result {
                    Some(Ok(resp_value)) => {
                        let response =
                            dispatch(
                                &store,
                                resp_value,
                            )
                            .await;

                        if let Err(err) =
                            framed.send(response).await
                        {
                            error!("{err}");
                            break;
                        }
                    }

                    Some(Err(err)) => {
                        error!("{err}");
                        break;
                    }

                    None => {
                        break;
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(store, frame))]
pub async fn dispatch(store: &Arc<Store>, frame: RespValue) -> RespValue {
    let args = match frame {
        RespValue::Array(resp_values) => resp_values,
        _ => return RespValue::Error(Bytes::from_static(b"ERR protocol error")),
    };

    match parse_command(args) {
        Ok(cmd) => execute(store.as_ref(), cmd),
        Err(err) => {
            warn!("Command parse error: {err}");
            RespValue::Error(Bytes::from(format!("ERR {err}")))
        }
    }
}
