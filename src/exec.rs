use bytes::Bytes;

use crate::cmd::Command;
use crate::codec::RespValue;

pub fn execute(cmd: Command) -> RespValue {
    match cmd {
        Command::Ping(msg) => match msg {
            Some(msg) => RespValue::BulkString(msg),
            None => RespValue::SimpleString(Bytes::from_static(b"PONG")),
        },
        Command::Echo(msg) => RespValue::BulkString(msg),
        Command::Set { key, value, ex } => RespValue::SimpleString(Bytes::from_static(b"Ok")),
        Command::Get(_key) => RespValue::Null,
        Command::Del(_items) => RespValue::Integer(0),
        Command::Exists(_items) => RespValue::Integer(0),
        Command::Incr(_inc) => RespValue::Integer(0),
        Command::Expire(_exps, _) => RespValue::Integer(0),
        Command::Ttl(_ttl) => RespValue::Integer(-1),
        Command::CommandDocs => RespValue::Array(vec![]),
    }
}
