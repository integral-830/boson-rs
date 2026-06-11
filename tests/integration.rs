use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use futures::{future::join_all, SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_util::codec::Framed;

use boson_rs::{
    codec::{RespCodec, RespValue},
    server::run_server,
    store::Store,
};

async fn start_server() -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let port = listener.local_addr().unwrap().port();

    let store = Arc::new(Store::new());

    let handle = tokio::spawn(async move {
        run_server(listener, store).await.unwrap();
    });

    (port, handle)
}

async fn connect(port: u16) -> Framed<TcpStream, RespCodec> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();

    Framed::new(stream, RespCodec)
}

fn bulk(s: &str) -> RespValue {
    RespValue::BulkString(Bytes::copy_from_slice(s.as_bytes()))
}

#[tokio::test]
async fn ping() {
    let (port, _server) = start_server().await;

    let mut client = connect(port).await;

    client
        .send(RespValue::Array(vec![bulk("PING")]))
        .await
        .unwrap();

    let response = client.next().await.unwrap().unwrap();

    assert_eq!(
        response,
        RespValue::SimpleString(Bytes::from_static(b"PONG"),)
    );
}

#[tokio::test]
async fn set_get() {
    let (port, _server) = start_server().await;

    let mut client = connect(port).await;

    client
        .send(RespValue::Array(vec![
            bulk("SET"),
            bulk("foo"),
            bulk("bar"),
        ]))
        .await
        .unwrap();

    let _ = client.next().await.unwrap().unwrap();

    client
        .send(RespValue::Array(vec![bulk("GET"), bulk("foo")]))
        .await
        .unwrap();

    let response = client.next().await.unwrap().unwrap();

    assert_eq!(response, RespValue::BulkString(Bytes::from_static(b"bar"),));
}

#[tokio::test]
async fn ttl_expiry() {
    let (port, _server) = start_server().await;

    let mut client = connect(port).await;

    client
        .send(RespValue::Array(vec![
            bulk("SET"),
            bulk("foo"),
            bulk("bar"),
            bulk("EX"),
            bulk("1"),
        ]))
        .await
        .unwrap();

    let _ = client.next().await.unwrap().unwrap();

    tokio::time::sleep(Duration::from_millis(1100)).await;

    client
        .send(RespValue::Array(vec![bulk("GET"), bulk("foo")]))
        .await
        .unwrap();

    let response = client.next().await.unwrap().unwrap();

    assert_eq!(response, RespValue::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn concurrent_connections() {
    let (port, _server) = start_server().await;

    let mut tasks = Vec::new();

    for worker in 0..10 {
        let task = tokio::spawn(async move {
            let mut client = connect(port).await;

            for i in 0..100 {
                let key = format!("k{worker}-{i}");

                let value = format!("v{worker}-{i}");

                client
                    .send(RespValue::Array(vec![
                        bulk("SET"),
                        bulk(&key),
                        bulk(&value),
                    ]))
                    .await
                    .unwrap();

                let _ = client.next().await.unwrap().unwrap();

                client
                    .send(RespValue::Array(vec![bulk("GET"), bulk(&key)]))
                    .await
                    .unwrap();

                let response = client.next().await.unwrap().unwrap();

                assert_eq!(
                    response,
                    RespValue::BulkString(Bytes::copy_from_slice(value.as_bytes(),),),
                );
            }
        });

        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        result.unwrap();
    }
}
