use std::sync::Arc;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, warn};

use crate::exec::execute;
use crate::{
    cmd::parse_command,
    codec::{RespCodec, RespValue},
    store::Store,
};

#[tracing::instrument(skip(stream,store),
    fields(peer_addr = ?stream.peer_addr().ok())
)]
pub async fn handle(stream: TcpStream, store: Arc<Store>) {
    let mut framed = Framed::new(stream, RespCodec);

    while let Some(frame) = framed.next().await {
        match frame {
            Ok(resp_value) => {
                let response = dispatch(&store, resp_value).await;
                if let Err(err) = framed.send(response).await {
                    error!("Send error: {err}");
                    break;
                }
            }
            Err(err) => {
                error!("Decode error: {err}");
                break;
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
