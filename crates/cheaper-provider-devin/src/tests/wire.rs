use crate::wire::*;

#[test]
fn varint_roundtrip() {
    assert_eq!(encode_varint(0), vec![0x00]);
    assert_eq!(encode_varint(1), vec![0x01]);
    assert_eq!(encode_varint(127), vec![0x7f]);
    // TS test: 300 → [0xac, 0x02]
    assert_eq!(encode_varint(300), vec![0xac, 0x02]);
    // u64::MAX は 10 byte。
    assert_eq!(encode_varint(u64::MAX).len(), 10);
    for v in [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX] {
        let enc = encode_varint(v);
        let (dec, next) = decode_varint(&enc, 0).expect("decode");
        assert_eq!(dec, v);
        assert_eq!(next, enc.len());
    }
}

#[test]
fn varint_truncated_is_error() {
    // 0x80 は continuation bit 立ちで末尾切れ。
    assert_eq!(decode_varint(&[0x80], 0), Err(DecodeError::TruncatedVarint));
}

#[test]
fn tag_and_string_fields() {
    // field 1 wire 2 → tag 0x0a; "hi" → len 2。
    assert_eq!(encode_string(1, "hi"), vec![0x0a, 0x02, b'h', b'i']);
    // field 21 string tag は varint 0xaa 0x01（(21<<3)|2 = 170）。
    let tagged = encode_string(21, "x");
    assert_eq!(&tagged[..2], &[0xaa, 0x01]);
}

#[test]
fn varint_field_and_fixed64() {
    // field 2 wire 0 varint 5。
    assert_eq!(encode_varint_field(2, 5), vec![0x10, 0x05]);
    // field 5 wire 1 fixed64 = f64 little-endian 0.7。
    let b = encode_fixed64_field(5, 0.7);
    assert_eq!(b[0], (5 << 3 | 1) as u8);
    assert_eq!(&b[1..], &0.7f64.to_le_bytes());
}

#[test]
fn timestamp_body_skips_zero_nanos() {
    // 1700_000 ms → seconds 1700、nanos 0 は省略。
    let body = encode_timestamp_body(1_700_000);
    assert_eq!(body, encode_varint_field(1, 1700));
    // 1_500_123 ms → nanos 123_000_000。
    let body = encode_timestamp_body(1_500_123);
    let fields: Vec<_> = iter_fields(&body).collect::<Result<_, _>>().unwrap();
    assert_eq!(fields[0].varint(), Some(1500));
    assert_eq!(fields[1].num, 2);
    assert_eq!(fields[1].varint(), Some(123_000_000));
}

#[test]
fn iter_fields_walks_all_wire_types() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&encode_varint_field(1, 42));
    buf.extend_from_slice(&encode_string(2, "s"));
    buf.extend_from_slice(&encode_fixed64_field(3, 1.5));
    buf.extend_from_slice(&encode_message(4, &encode_varint_field(1, 7)));
    let fields: Vec<ProtoField> = iter_fields(&buf).collect::<Result<_, _>>().unwrap();
    assert_eq!(fields.len(), 4);
    assert_eq!((fields[0].num, fields[0].varint()), (1, Some(42)));
    assert_eq!(
        (fields[1].num, fields[1].string().as_deref()),
        (2, Some("s"))
    );
    assert_eq!(fields[2].num, 3);
    assert_eq!(fields[2].wire, 1);
    let inner: Vec<ProtoField> = iter_fields(fields[3].bytes().unwrap())
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(inner[0].varint(), Some(7));
}

#[test]
fn iter_fields_stops_on_unknown_wire_type() {
    // tag (1<<3)|3 = 0x0b は wire type 3（group）→ silent 終端。
    let buf = [0x0b, 0x00];
    assert_eq!(iter_fields(&buf).count(), 0);
}

#[test]
fn connect_frame_uncompressed_layout() {
    let framed = frame_connect_stream(b"abc", false);
    assert_eq!(&framed[..5], &[0x00, 0x00, 0x00, 0x00, 0x03]);
    assert_eq!(&framed[5..], b"abc");
}

#[test]
fn connect_frame_compressed_roundtrip() {
    let framed = frame_connect_stream(b"hello devin", true);
    assert_eq!(framed[0] & 0x01, 0x01);
    let len = u32::from_be_bytes([framed[1], framed[2], framed[3], framed[4]]) as usize;
    assert_eq!(len, framed.len() - 5);
    assert_eq!(gunzip(&framed[5..]).unwrap(), b"hello devin");
}
