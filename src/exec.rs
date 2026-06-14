use bytes::Bytes;

use crate::cmd::Command;
use crate::codec::RespValue;
use crate::store::Store;

const PONG: &[u8] = b"PONG";
const OK: &[u8] = b"OK";

#[tracing::instrument(
    skip(store),
    fields(command = ?cmd)
)]
pub fn execute(store: &Store, cmd: Command) -> RespValue {
    match cmd {
        Command::Ping(msg) => match msg {
            Some(msg) => RespValue::SimpleString(msg),
            None => RespValue::SimpleString(Bytes::from_static(PONG)),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set { key, value, ex } => {
            store.set(key, value, ex);
            RespValue::SimpleString(Bytes::from_static(OK))
        }
        Command::Get(key) => match store.get(&key) {
            Some(value) => RespValue::BulkString(value),
            None => RespValue::Null,
        },
        Command::Del(keys) => RespValue::Integer(store.del(&keys)),
        Command::Exists(keys) => RespValue::Integer(store.exists(&keys)),
        Command::Incr(key) => match store.incr(key) {
            Ok(value) => RespValue::Integer(value),
            Err(err) => RespValue::Error(Bytes::from(err)),
        },
        Command::Expire(key, secs) => RespValue::Integer(store.expire(&key, secs)),
        Command::Ttl(key) => RespValue::Integer(store.ttl(&key)),
        Command::CommandDocs | Command::ConfigGet => RespValue::Array(vec![]),
        Command::MGet(keys) => {
            let resp_values = store
                .mget(&keys)
                .into_iter()
                .map(|val| match val {
                    Some(value) => RespValue::BulkString(value),
                    None => RespValue::Null,
                })
                .collect();
            RespValue::Array(resp_values)
        }
    }
}

#[cfg(test)]
mod tests {

    use bytes::Bytes;

    use crate::cmd::Command;
    use crate::codec::RespValue;
    use crate::exec::execute;
    use crate::store::Store;

    #[test]
    fn execute_mget() {
        let store = Store::new();

        store.set(Bytes::from("key1"), Bytes::from("value1"), None);

        store.set(Bytes::from("key2"), Bytes::from("value2"), None);

        let resp = execute(
            &store,
            Command::MGet(vec![
                Bytes::from("key1"),
                Bytes::from("key2"),
                Bytes::from("missing"),
            ]),
        );

        assert_eq!(
            resp,
            RespValue::Array(vec![
                RespValue::BulkString(Bytes::from("value1")),
                RespValue::BulkString(Bytes::from("value2")),
                RespValue::Null,
            ])
        );
    }
}
