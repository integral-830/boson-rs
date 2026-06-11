use bytes::Bytes;

use crate::cmd::Command;
use crate::codec::RespValue;
use crate::store::Store;

#[tracing::instrument(
    skip(store),
    fields(command = ?cmd)
)]
pub fn execute(store: &Store, cmd: Command) -> RespValue {
    match cmd {
        Command::Ping(msg) => match msg {
            Some(msg) => RespValue::SimpleString(msg),
            None => RespValue::SimpleString(Bytes::from_static(b"PONG")),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set { key, value, ex } => {
            store.set(key, value, ex);
            RespValue::SimpleString(Bytes::from_static(b"OK"))
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
    }
}

// #[cfg(test)]
// mod tests {
//     use bytes::Bytes;
//
//     use crate::cmd::Command;
//     use crate::codec::RespValue;
//     use crate::exec::execute;
//
//     #[test]
//     fn execute_ping() {
//         assert_eq!(
//             execute(Command::Ping(None)),
//             RespValue::SimpleString(Bytes::from_static(b"PONG"))
//         );
//     }
//
//     #[test]
//     fn execute_set() {
//         assert_eq!(
//             execute(Command::Set {
//                 key: Bytes::from("k"),
//                 value: Bytes::from("v"),
//                 ex: None,
//             }),
//             RespValue::SimpleString(Bytes::from_static(b"OK"))
//         );
//     }
//
//     #[test]
//     fn execute_get() {
//         assert_eq!(execute(Command::Get(Bytes::from("k"))), RespValue::Null);
//     }
//
//     #[test]
//     fn execute_command_docs() {
//         assert_eq!(execute(Command::CommandDocs), RespValue::Array(vec![]));
//     }
// }
