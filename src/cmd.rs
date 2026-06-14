use bytes::Bytes;
use thiserror::Error;

use crate::codec::RespValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Ping(Option<Bytes>),
    Echo(Bytes),
    Set {
        key: Bytes,
        value: Bytes,
        ex: Option<u64>,
    },
    Get(Bytes),
    MGet(Vec<Bytes>),
    Del(Vec<Bytes>),
    Exists(Vec<Bytes>),
    Incr(Bytes),
    Expire(Bytes, u64),
    Ttl(Bytes),
    CommandDocs,
    ConfigGet,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("wrong number of args for {cmd} : {got} :: expected:{expected}")]
    WrongArity {
        cmd: &'static str,
        got: usize,
        expected: &'static str,
    },

    #[error("expected RESP BulkString args")]
    NotBulkString,

    #[error("invalid arg: {0:?}")]
    InvalidArg(Bytes),

    #[error("Unknown command: {0:?}")]
    UnknownCommand(Bytes),
}

fn check_bulk(value: RespValue) -> Result<Bytes, CommandError> {
    match value {
        RespValue::SimpleString(bytes) | RespValue::BulkString(bytes) => Ok(bytes),
        _ => Err(CommandError::NotBulkString),
    }
}

pub fn parse_command(args: Vec<RespValue>) -> Result<Command, CommandError> {
    if args.is_empty() {
        return Err(CommandError::WrongArity {
            cmd: "UNKNOWN",
            got: 0,
            expected: "expected <command> [args]",
        });
    }

    let mut bytes_iter = args.into_iter();
    let cmd_bytes = check_bulk(bytes_iter.next().unwrap())?;

    match cmd_bytes.to_ascii_uppercase().as_slice() {
        b"PING" => {
            let remaining_byte: Vec<_> = bytes_iter.collect();
            match remaining_byte.len() {
                0 => Ok(Command::Ping(None)),
                1 => Ok(Command::Ping(Some(check_bulk(
                    remaining_byte.into_iter().next().unwrap(),
                )?))),
                n => Err(CommandError::WrongArity {
                    cmd: "PING",
                    got: n + 1,
                    expected: "PING [message]",
                }),
            }
        }
        b"ECHO" => {
            let remaining_bytes: Vec<_> = bytes_iter.collect();
            if remaining_bytes.len() != 1 {
                return Err(CommandError::WrongArity {
                    cmd: "ECHO",
                    got: remaining_bytes.len() + 1,
                    expected: "ECHO message",
                });
            }
            Ok(Command::Echo(check_bulk(
                remaining_bytes.into_iter().next().unwrap(),
            )?))
        }
        b"GET" => {
            let remaining_bytes: Vec<_> = bytes_iter.collect();
            if remaining_bytes.len() != 1 {
                return Err(CommandError::WrongArity {
                    cmd: "GET",
                    got: remaining_bytes.len() + 1,
                    expected: "GET key",
                });
            }
            Ok(Command::Get(check_bulk(
                remaining_bytes.into_iter().next().unwrap(),
            )?))
        }
        b"MGET" => {
            let keys = bytes_iter.map(check_bulk).collect::<Result<Vec<_>, _>>()?;
            if keys.is_empty() {
                return Err(CommandError::WrongArity {
                    cmd: "MGET",
                    got: 1,
                    expected: "MGET key [key ...]",
                });
            }
            Ok(Command::MGet(keys))
        }
        b"SET" => {
            let remaining_bytes: Vec<_> = bytes_iter.collect();
            if remaining_bytes.len() < 2 {
                return Err(CommandError::WrongArity {
                    cmd: "SET",
                    got: remaining_bytes.len() + 1,
                    expected: "SET key value [EX seconds|PX miliseconds]",
                });
            }
            let mut iter = remaining_bytes.into_iter();
            let key = check_bulk(iter.next().unwrap())?;
            let value = check_bulk(iter.next().unwrap())?;
            let mut ex = None;
            while let Some(opt) = iter.next() {
                let opt = check_bulk(opt)?;
                let Some(arg) = iter.next() else {
                    break;
                };
                let arg = check_bulk(arg)?;
                match opt.to_ascii_uppercase().as_slice() {
                    b"EX" => {
                        let secs = String::from_utf8_lossy(&arg)
                            .parse::<u64>()
                            .map_err(|_| CommandError::InvalidArg(arg.clone()))?;
                        ex = Some(secs);
                    }
                    b"PX" => {
                        let ms = String::from_utf8_lossy(&arg)
                            .parse::<u64>()
                            .map_err(|_| CommandError::InvalidArg(arg.clone()))?;
                        ex = Some(ms / 1000);
                    }
                    _ => {}
                }
            }
            Ok(Command::Set { key, value, ex })
        }
        b"DEL" => {
            let keys = bytes_iter.map(check_bulk).collect::<Result<Vec<_>, _>>()?;
            if keys.is_empty() {
                return Err(CommandError::WrongArity {
                    cmd: "DEL",
                    got: 1,
                    expected: "DEL key [key ...]",
                });
            }
            Ok(Command::Del(keys))
        }
        b"EXISTS" => {
            let keys = bytes_iter.map(check_bulk).collect::<Result<Vec<_>, _>>()?;
            if keys.is_empty() {
                return Err(CommandError::WrongArity {
                    cmd: "EXISTS",
                    got: 1,
                    expected: "EXISTS key [key ...]",
                });
            }
            Ok(Command::Exists(keys))
        }
        b"INCR" => {
            let remaining_bytes: Vec<_> = bytes_iter.collect();
            if remaining_bytes.len() != 1 {
                return Err(CommandError::WrongArity {
                    cmd: "INCR",
                    got: remaining_bytes.len() + 1,
                    expected: "INCR key",
                });
            }
            Ok(Command::Incr(check_bulk(
                remaining_bytes.into_iter().next().unwrap(),
            )?))
        }
        b"EXPIRE" => {
            let remaining: Vec<_> = bytes_iter.collect();

            if remaining.len() != 2 {
                return Err(CommandError::WrongArity {
                    cmd: "EXPIRE",
                    got: remaining.len() + 1,
                    expected: "EXPIRE key seconds",
                });
            }

            let mut it = remaining.into_iter();

            let key = check_bulk(it.next().unwrap())?;

            let ttl_arg = check_bulk(it.next().unwrap())?;

            let ttl = String::from_utf8_lossy(&ttl_arg)
                .parse::<u64>()
                .map_err(|_| CommandError::InvalidArg(ttl_arg.clone()))?;

            Ok(Command::Expire(key, ttl))
        }
        b"TTL" => {
            let remaining_bytes: Vec<_> = bytes_iter.collect();
            if remaining_bytes.len() != 1 {
                return Err(CommandError::WrongArity {
                    cmd: "TTL",
                    got: remaining_bytes.len() + 1,
                    expected: "TTL key",
                });
            }
            Ok(Command::Ttl(check_bulk(
                remaining_bytes.into_iter().next().unwrap(),
            )?))
        }
        b"COMMAND" => Ok(Command::CommandDocs),
        b"CONFIG" => Ok(Command::ConfigGet),
        _ => Err(CommandError::UnknownCommand(cmd_bytes)),
    }
}

#[cfg(test)]
mod tests {

    use bytes::Bytes;

    use crate::cmd::{parse_command, Command, CommandError};
    use crate::codec::RespValue;

    fn bulk(s: &str) -> RespValue {
        RespValue::BulkString(Bytes::from(s.to_owned()))
    }

    #[test]
    fn parse_ping() {
        let cmd = parse_command(vec![bulk("PING")]).unwrap();

        assert!(matches!(cmd, Command::Ping(None)));
    }

    #[test]
    fn parse_ping_message() {
        let cmd = parse_command(vec![bulk("PING"), bulk("hello")]).unwrap();

        assert_eq!(cmd, Command::Ping(Some(Bytes::from("hello"))));
    }

    #[test]
    fn parse_echo() {
        let cmd = parse_command(vec![bulk("ECHO"), bulk("hello")]).unwrap();

        assert_eq!(cmd, Command::Echo(Bytes::from("hello")));
    }

    #[test]
    fn parse_get() {
        let cmd = parse_command(vec![bulk("GET"), bulk("foo")]).unwrap();

        assert_eq!(cmd, Command::Get(Bytes::from("foo")));
    }

    #[test]
    fn parse_set() {
        let cmd = parse_command(vec![bulk("SET"), bulk("key"), bulk("value")]).unwrap();

        assert_eq!(
            cmd,
            Command::Set {
                key: Bytes::from("key"),
                value: Bytes::from("value"),
                ex: None,
            }
        );
    }

    #[test]
    fn parse_set_ex() {
        let cmd = parse_command(vec![
            bulk("SET"),
            bulk("key"),
            bulk("value"),
            bulk("EX"),
            bulk("60"),
        ])
        .unwrap();

        assert_eq!(
            cmd,
            Command::Set {
                key: Bytes::from("key"),
                value: Bytes::from("value"),
                ex: Some(60),
            }
        );
    }

    #[test]
    fn parse_set_px() {
        let cmd = parse_command(vec![
            bulk("SET"),
            bulk("key"),
            bulk("value"),
            bulk("PX"),
            bulk("5000"),
        ])
        .unwrap();

        assert_eq!(
            cmd,
            Command::Set {
                key: Bytes::from("key"),
                value: Bytes::from("value"),
                ex: Some(5),
            }
        );
    }

    #[test]
    fn parse_del() {
        let cmd = parse_command(vec![bulk("DEL"), bulk("a"), bulk("b")]).unwrap();

        assert_eq!(cmd, Command::Del(vec![Bytes::from("a"), Bytes::from("b"),]));
    }

    #[test]
    fn parse_exists() {
        let cmd = parse_command(vec![bulk("EXISTS"), bulk("a"), bulk("b")]).unwrap();

        assert_eq!(
            cmd,
            Command::Exists(vec![Bytes::from("a"), Bytes::from("b"),])
        );
    }

    #[test]
    fn parse_incr() {
        let cmd = parse_command(vec![bulk("INCR"), bulk("counter")]).unwrap();

        assert_eq!(cmd, Command::Incr(Bytes::from("counter")));
    }

    #[test]
    fn parse_expire() {
        let cmd = parse_command(vec![bulk("EXPIRE"), bulk("key"), bulk("120")]).unwrap();

        assert_eq!(cmd, Command::Expire(Bytes::from("key"), 120,));
    }

    #[test]
    fn parse_ttl() {
        let cmd = parse_command(vec![bulk("TTL"), bulk("key")]).unwrap();

        assert_eq!(cmd, Command::Ttl(Bytes::from("key")));
    }

    #[test]
    fn parse_command_docs() {
        let cmd = parse_command(vec![bulk("COMMAND"), bulk("DOCS")]).unwrap();

        assert_eq!(cmd, Command::CommandDocs);
    }

    #[test]
    fn command_names_are_case_insensitive() {
        let lower = parse_command(vec![bulk("get"), bulk("foo")]).unwrap();

        let upper = parse_command(vec![bulk("GET"), bulk("foo")]).unwrap();

        assert_eq!(lower, upper,);
    }

    #[test]
    fn get_wrong_arity() {
        let err = parse_command(vec![bulk("GET")]).unwrap_err();

        assert!(matches!(err, CommandError::WrongArity { cmd: "GET", .. }));
    }
}
