use boson_rs::codec::{RespCodec, RespValue};

use bytes::{Bytes, BytesMut};
use proptest::prelude::*;
use tokio_util::codec::{Decoder, Encoder};

fn simple_line_bytes() -> impl Strategy<Value = Bytes> {
    prop::collection::vec(
        any::<u8>().prop_filter("RESP SimpleString/Error cannot contain CR or LF", |b| {
            *b != b'\r' && *b != b'\n'
        }),
        0..64,
    )
    .prop_map(Bytes::from)
}

fn resp_value_strategy() -> impl Strategy<Value = RespValue> {
    let leaf = prop_oneof![
        simple_line_bytes().prop_map(RespValue::SimpleString),
        simple_line_bytes().prop_map(RespValue::Error),
        any::<i64>().prop_map(RespValue::Integer),
        prop::collection::vec(any::<u8>(), 0..64)
            .prop_map(|v| { RespValue::BulkString(Bytes::from(v)) }),
        Just(RespValue::Null),
    ];

    leaf.prop_recursive(3, 64, 8, |inner| {
        prop::collection::vec(inner, 0..8).prop_map(RespValue::Array)
    })
}

proptest! {
    #[test]
    fn resp_roundtrip(
        value in resp_value_strategy()
    ) {
        let mut codec = RespCodec;

        let mut buf = BytesMut::new();

        codec
            .encode(
                value.clone(),
                &mut buf,
            )
            .unwrap();

        let decoded = codec
            .decode(&mut buf)
            .unwrap()
            .unwrap();

        prop_assert_eq!(
            value,
            decoded
        );

        prop_assert!(
            buf.is_empty()
        );
    }
}

proptest! {
    #[test]
    fn decode_never_panics(
        data in prop::collection::vec(
            any::<u8>(),
            0..1024,
        )
    ) {
        let mut codec = RespCodec;
        let mut buf = BytesMut::from(
            data.as_slice()
        );

        let _ = codec.decode(&mut buf);
    }
}
