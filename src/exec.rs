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
        Command::Set { key, value, ex } => RespValue::SimpleString(Bytes::from_static(b"OK")),
        Command::Get(_key) => RespValue::Null,
        Command::Del(_items) => RespValue::Integer(0),
        Command::Exists(_items) => RespValue::Integer(0),
        Command::Incr(_inc) => RespValue::Integer(0),
        Command::Expire(_exps, _) => RespValue::Integer(0),
        Command::Ttl(_ttl) => RespValue::Integer(-1),
        Command::CommandDocs => RespValue::Array(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::cmd::Command;
    use crate::codec::RespValue;
    use crate::exec::execute;

    #[test]
    fn execute_ping() {
        assert_eq!(
            execute(Command::Ping(None)),
            RespValue::SimpleString(Bytes::from_static(b"PONG"))
        );
    }

    #[test]
    fn execute_set() {
        assert_eq!(
            execute(Command::Set {
                key: Bytes::from("k"),
                value: Bytes::from("v"),
                ex: None,
            }),
            RespValue::SimpleString(Bytes::from_static(b"OK"))
        );
    }

    #[test]
    fn execute_get() {
        assert_eq!(execute(Command::Get(Bytes::from("k"))), RespValue::Null);
    }

    #[test]
    fn execute_command_docs() {
        assert_eq!(execute(Command::CommandDocs), RespValue::Array(vec![]));
    }
}
