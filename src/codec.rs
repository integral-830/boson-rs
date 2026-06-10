use bytes::{Buf, BufMut, Bytes};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(Bytes),
    BulkString(Bytes),
    Integer(i64),
    Array(Vec<RespValue>),
    Error(Bytes),
    Null,
}

#[derive(Debug, Error)]
pub enum RespError {
    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("invalid RESP prefix: {0:#x}")]
    InvalidPrefix(u8),

    #[error("invalid RESP length")]
    InvalidLength,

    #[error("invalid integer")]
    InvalidInteger,

    #[error("integer parse error")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("incomplete frame")]
    Incomplete,
}

fn get_crlf_from(buf: &[u8], start: usize) -> Option<usize> {
    buf.get(start..)?
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|p| start + p)
}

fn decode_simple_string_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    let start = *pos;

    let crlf_pos = match get_crlf_from(buf, start) {
        Some(pos_c) => pos_c,
        None => return Ok(None),
    };

    let data = Bytes::copy_from_slice(&buf[start + 1..crlf_pos]);

    *pos = crlf_pos + 2;

    Ok(Some(RespValue::SimpleString(data)))
}

fn decode_error_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    let start = *pos;

    let crlf_pos = match get_crlf_from(buf, start) {
        Some(pos_c) => pos_c,
        None => return Ok(None),
    };

    let data = Bytes::copy_from_slice(&buf[start + 1..crlf_pos]);

    *pos = crlf_pos + 2;

    Ok(Some(RespValue::Error(data)))
}

fn decode_integer_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    let start = *pos;

    let crlf_pos = match get_crlf_from(buf, start) {
        Some(pos_c) => pos_c,
        None => return Ok(None),
    };

    let value: i64 = std::str::from_utf8(&buf[start + 1..crlf_pos])
        .map_err(|_| RespError::InvalidInteger)?
        .parse()?;

    *pos = crlf_pos + 2;

    Ok(Some(RespValue::Integer(value)))
}

fn decode_bulk_string_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    let start = *pos;

    let header_end = match get_crlf_from(buf, start) {
        Some(pos_c) => pos_c,
        None => return Ok(None),
    };

    let len: i64 = std::str::from_utf8(&buf[start + 1..header_end])
        .map_err(|_| RespError::InvalidLength)?
        .parse()?;

    if len == -1 {
        *pos = header_end + 2;

        return Ok(Some(RespValue::Null));
    }

    if len < -1 {
        return Err(RespError::InvalidLength);
    }

    let payload_start = header_end + 2;

    let payload_end = payload_start + len as usize;

    let total_consumed = payload_end + 2;

    if buf.len() < total_consumed {
        return Ok(None);
    }

    let data = Bytes::copy_from_slice(&buf[payload_start..payload_end]);

    *pos = total_consumed;

    Ok(Some(RespValue::BulkString(data)))
}

fn decode_array_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    let start = *pos;

    let header_end = match get_crlf_from(buf, start) {
        Some(pos_c) => pos_c,
        None => return Ok(None),
    };

    let len: i64 = std::str::from_utf8(&buf[start + 1..header_end])
        .map_err(|_| RespError::InvalidLength)?
        .parse()?;

    if len == -1 {
        *pos = header_end + 2;

        return Ok(Some(RespValue::Null));
    }

    if len < -1 {
        return Err(RespError::InvalidLength);
    }

    *pos = header_end + 2;

    let mut resp_values_array = Vec::with_capacity(len as usize);

    for _ in 0..len {
        let value = match decode_at(buf, pos)? {
            Some(resp_value) => resp_value,
            None => {
                *pos = start;
                return Ok(None);
            }
        };
        resp_values_array.push(value);
    }

    Ok(Some(RespValue::Array(resp_values_array)))
}

fn decode_at(buf: &[u8], pos: &mut usize) -> Result<Option<RespValue>, RespError> {
    if *pos >= buf.len() {
        return Ok(None);
    }

    match buf[*pos] {
        b'+' => decode_simple_string_at(buf, pos),
        b'-' => decode_error_at(buf, pos),
        b':' => decode_integer_at(buf, pos),
        b'$' => decode_bulk_string_at(buf, pos),
        b'*' => decode_array_at(buf, pos),
        other => Err(RespError::InvalidPrefix(other)),
    }
}

pub struct RespCodec;

impl Decoder for RespCodec {
    type Item = RespValue;

    type Error = RespError;

    fn decode(&mut self, buf: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        let mut pos = 0;

        let value = match decode_at(&buf[..], &mut pos)? {
            Some(val) => val,
            None => return Ok(None),
        };

        buf.advance(pos);
        Ok(Some(value))
    }
}

impl Encoder<RespValue> for RespCodec {
    type Error = RespError;

    fn encode(&mut self, item: RespValue, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(estimate_size(&item));

        match item {
            RespValue::SimpleString(bytes) => {
                dst.put_u8(b'+');
                dst.put_slice(&bytes);
                dst.put_slice(b"\r\n");
            }
            RespValue::BulkString(bytes) => {
                dst.put_u8(b'$');

                let mut buf = itoa::Buffer::new();
                let str = buf.format(bytes.len());

                dst.put_slice(str.as_bytes());
                dst.put_slice(b"\r\n");
                dst.put_slice(&bytes);
                dst.put_slice(b"\r\n");
            }
            RespValue::Integer(value) => {
                dst.put_u8(b':');
                let mut buf = itoa::Buffer::new();
                let str = buf.format(value);
                dst.put_slice(str.as_bytes());
                dst.put_slice(b"\r\n");
            }
            RespValue::Array(resp_values) => {
                dst.put_u8(b'*');
                let mut buf = itoa::Buffer::new();
                let str = buf.format(resp_values.len());
                dst.put_slice(str.as_bytes());
                dst.put_slice(b"\r\n");
                for item in resp_values {
                    self.encode(item, dst)?;
                }
            }
            RespValue::Error(bytes) => {
                dst.put_u8(b'-');
                dst.put_slice(&bytes);
                dst.put_slice(b"\r\n");
            }
            RespValue::Null => {
                dst.put_slice(b"$-1\r\n");
            }
        }
        Ok(())
    }
}

fn estimate_size(value: &RespValue) -> usize {
    match value {
        RespValue::SimpleString(bytes) | RespValue::Error(bytes) => 3 + bytes.len(),
        RespValue::BulkString(bytes) => 20 + bytes.len(),
        RespValue::Integer(_) => 32,
        RespValue::Array(resp_values) => 10 + resp_values.iter().map(estimate_size).sum::<usize>(),
        RespValue::Null => 8,
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

    use super::*;

    #[test]
    fn assert_crlf() {
        assert_eq!(get_crlf_from(b"+Ping\r\n", 0), Some(5));
    }

    #[test]
    fn assert_no_crlf() {
        assert_eq!(get_crlf_from(b"+Ping", 0), None);
    }

    #[test]
    fn assert_no_crlf_on_empty_buffer() {
        assert_eq!(get_crlf_from(b"", 0), None);
    }

    fn mk(s: &str) -> BytesMut {
        BytesMut::from(s.as_bytes())
    }

    #[test]
    fn decode_simple_string() {
        let mut codec = RespCodec;
        let mut buf = mk("+OK\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::SimpleString(Bytes::from("OK"))));

        assert!(buf.is_empty());
    }

    #[test]
    fn decode_error_with_spaces() {
        let mut codec = RespCodec;
        let mut buf = mk("-ERR unknown command\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(
            value,
            Some(RespValue::Error(Bytes::from("ERR unknown command")))
        );

        assert!(buf.is_empty());
    }

    #[test]
    fn decode_integer_positive() {
        let mut codec = RespCodec;
        let mut buf = mk(":42\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::Integer(42)));
    }

    #[test]
    fn decode_integer_negative() {
        let mut codec = RespCodec;
        let mut buf = mk(":-42\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::Integer(-42)));
    }

    #[test]
    fn decode_integer_zero() {
        let mut codec = RespCodec;
        let mut buf = mk(":0\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::Integer(0)));
    }

    #[test]
    fn decode_bulk_string() {
        let mut codec = RespCodec;
        let mut buf = mk("$5\r\nhello\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::BulkString(Bytes::from("hello"))));
    }

    #[test]
    fn decode_bulk_string_binary() {
        let mut codec = RespCodec;

        let mut buf = BytesMut::new();

        buf.put_slice(b"$4\r\n\x00\x01\x02\xff\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(
            value,
            Some(RespValue::BulkString(Bytes::from_static(
                b"\x00\x01\x02\xff"
            )))
        );
    }

    #[test]
    fn decode_null_bulk_string() {
        let mut codec = RespCodec;
        let mut buf = mk("$-1\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::Null));
    }

    #[test]
    fn decode_array_of_strings() {
        let mut codec = RespCodec;

        let mut buf = mk("*3\r\n\
             +one\r\n\
             +two\r\n\
             +three\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(
            value,
            Some(RespValue::Array(vec![
                RespValue::SimpleString(Bytes::from("one")),
                RespValue::SimpleString(Bytes::from("two")),
                RespValue::SimpleString(Bytes::from("three")),
            ]))
        );
    }

    #[test]
    fn decode_nested_array() {
        let mut codec = RespCodec;

        let mut buf = mk("*2\r\n\
             +outer\r\n\
             *2\r\n\
             +inner1\r\n\
             +inner2\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(
            value,
            Some(RespValue::Array(vec![
                RespValue::SimpleString(Bytes::from("outer")),
                RespValue::Array(vec![
                    RespValue::SimpleString(Bytes::from("inner1")),
                    RespValue::SimpleString(Bytes::from("inner2")),
                ]),
            ]))
        );
    }

    #[test]
    fn partial_bulk_string() {
        let mut codec = RespCodec;

        let mut buf = mk("$5\r\nhel");

        let value = codec.decode(&mut buf).unwrap();

        assert!(value.is_none());

        assert_eq!(buf.len(), 7);

        buf.extend_from_slice(b"lo\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(value, Some(RespValue::BulkString(Bytes::from("hello"))));

        assert!(buf.is_empty());
    }

    #[test]
    fn partial_array() {
        let mut codec = RespCodec;

        let mut buf = mk("*2\r\n+OK\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert!(value.is_none());

        assert_eq!(buf, mk("*2\r\n+OK\r\n"));

        buf.extend_from_slice(b":42\r\n");

        let value = codec.decode(&mut buf).unwrap();

        assert_eq!(
            value,
            Some(RespValue::Array(vec![
                RespValue::SimpleString(Bytes::from("OK")),
                RespValue::Integer(42),
            ]))
        );
    }

    fn roundtrip(v: RespValue) {
        let mut codec = RespCodec;
        let mut buf = BytesMut::new();

        codec.encode(v.clone(), &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(v, decoded);

        assert!(buf.is_empty(), "decoder did not consume all bytes");
    }

    #[test]
    fn roundtrip_simple_string() {
        roundtrip(RespValue::SimpleString(Bytes::from("OK")));
    }

    #[test]
    fn roundtrip_error() {
        roundtrip(RespValue::Error(Bytes::from("ERR unknown command")));
    }

    #[test]
    fn roundtrip_integer_positive() {
        roundtrip(RespValue::Integer(42));
    }

    #[test]
    fn roundtrip_integer_zero() {
        roundtrip(RespValue::Integer(0));
    }

    #[test]
    fn roundtrip_bulk_string() {
        roundtrip(RespValue::BulkString(Bytes::from("hello")));
    }

    #[test]
    fn roundtrip_bulk_string_binary() {
        roundtrip(RespValue::BulkString(Bytes::from_static(
            b"\x00\x01\x02\xff",
        )));
    }

    #[test]
    fn roundtrip_null() {
        roundtrip(RespValue::Null);
    }

    #[test]
    fn roundtrip_nested_array() {
        roundtrip(RespValue::Array(vec![
            RespValue::SimpleString(Bytes::from("outer")),
            RespValue::Array(vec![
                RespValue::Integer(1),
                RespValue::Integer(2),
                RespValue::BulkString(Bytes::from("inner")),
            ]),
        ]));
    }

    #[test]
    fn bulk_string_with_crlf_payload() {
        let mut codec = RespCodec;

        let mut buf = BytesMut::from(b"$2\r\n\r\n\r\n".as_slice());

        let value = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(value, RespValue::BulkString(Bytes::from_static(b"\r\n")));
    }
}
