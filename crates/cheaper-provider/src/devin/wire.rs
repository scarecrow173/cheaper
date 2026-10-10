//! protobuf-lite wire encoding + Connect-RPC streaming frame（TS
//! `providers/devin/wire.ts` の port — vendored from pi-devin,
//! Copyright (c) 2026 Kashyab Ambarani, MIT License）。
//!
//! byte-level parity が contract: protobuf tag/length-delimited/varint/
//! fixed64(little-endian double) と Connect の `flags(1B) + len(4B BE) +
//! payload` frame を TS と同じ byte 列で組み立てる。

use std::io::{Read, Write};

/// varint 上限（u64 は 10 byte で収まる）。TS は BigInt で任意長だが
/// cheaper が流す値（timestamp・固定値）は全て u64 に収まる。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// `decodeVarint` が buffer 末尾に達した（TS `"truncated varint"`）。
    #[error("truncated varint")]
    TruncatedVarint,
    /// gzip payload の伸長失敗（TS `zlib.gunzipSync` throw）。
    #[error("gzip decode failed: {0}")]
    Gzip(String),
}

/// `encodeVarint` — unsigned LEB128。TS は negative input を
/// `RangeError` で reject する；Rust 側は `u64` を取るので負値は
/// 型で排除される。
pub fn encode_varint(value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut v = value;
    while v > 0x7f {
        bytes.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
    bytes.push(v as u8);
    bytes
}

/// `encodeTag` — `(field_num << 3) | wire` の varint。
pub fn encode_tag(field_num: u32, wire: u32) -> Vec<u8> {
    encode_varint(((field_num as u64) << 3) | wire as u64)
}

/// `encodeString` — length-delimited UTF-8 string field（wire type 2）。
pub fn encode_string(field_num: u32, s: &str) -> Vec<u8> {
    let buf = s.as_bytes();
    let mut out = encode_tag(field_num, 2);
    out.extend_from_slice(&encode_varint(buf.len() as u64));
    out.extend_from_slice(buf);
    out
}

/// `encodeMessage` — length-delimited nested message field（wire type 2）。
pub fn encode_message(field_num: u32, body: &[u8]) -> Vec<u8> {
    let mut out = encode_tag(field_num, 2);
    out.extend_from_slice(&encode_varint(body.len() as u64));
    out.extend_from_slice(body);
    out
}

/// `encodeVarintField` — varint field（wire type 0）。
pub fn encode_varint_field(field_num: u32, v: u64) -> Vec<u8> {
    let mut out = encode_tag(field_num, 0);
    out.extend_from_slice(&encode_varint(v));
    out
}

/// `encodeFixed64Field` — TS は `writeDoubleLE`（protobuf の double =
/// wire type 1、little-endian IEEE754）。fixed64 int では**ない**ことに注意。
pub fn encode_fixed64_field(field_num: u32, v: f64) -> Vec<u8> {
    let mut out = encode_tag(field_num, 1);
    out.extend_from_slice(&v.to_le_bytes());
    out
}

/// `encodeTimestampBody` — `Date.now()` 相当の epoch millis から
/// `{1: seconds, 2?: nanos}`（nanos は 0 のとき省略）。
pub fn encode_timestamp_body(now_ms: u64) -> Vec<u8> {
    let seconds = now_ms / 1000;
    let nanos = (now_ms % 1000) * 1_000_000;
    let mut out = encode_varint_field(1, seconds);
    if nanos > 0 {
        out.extend_from_slice(&encode_varint_field(2, nanos));
    }
    out
}

/// `decodeVarint` — `(value, next_offset)`。末尾途中で切れた varint は
/// TS 同様 error。
pub fn decode_varint(buf: &[u8], offset: usize) -> Result<(u64, usize), DecodeError> {
    let mut res: u64 = 0;
    let mut shift = 0u32;
    let mut i = offset;
    while i < buf.len() {
        let b = buf[i];
        i += 1;
        res |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok((res, i));
        }
        shift += 7;
        // u64 varint は最大 10 byte — それ以上は malformed。
        if shift >= 70 {
            return Err(DecodeError::TruncatedVarint);
        }
    }
    Err(DecodeError::TruncatedVarint)
}

/// `iterFields` が yield する 1 field。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtoField<'a> {
    pub num: u32,
    pub wire: u32,
    pub value: FieldValue<'a>,
}

impl ProtoField<'_> {
    /// length-delimited field の UTF-8 文字列（TS `value.toString("utf8")`
    /// — invalid UTF-8 は lossy）。
    pub fn string(&self) -> Option<String> {
        match &self.value {
            FieldValue::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
            FieldValue::Varint(_) => None,
        }
    }
    /// length-delimited field の生 byte。
    pub fn bytes(&self) -> Option<&[u8]> {
        match &self.value {
            FieldValue::Bytes(b) => Some(b),
            FieldValue::Varint(_) => None,
        }
    }
    /// varint field の値。
    pub fn varint(&self) -> Option<u64> {
        match &self.value {
            FieldValue::Varint(v) => Some(*v),
            FieldValue::Bytes(_) => None,
        }
    }
}

/// `iterFields` — wire type 0(varint)/1(64bit)/2(length-delimited)/
/// 5(32bit) を順に yield。TS 同様、unknown wire type・field 長の
/// 末尾超過は silent に iterator を終端させる。tag/length の varint が
/// truncated な場合のみ `Err` を 1 件 yield して終端（TS の throw 相当）。
pub fn iter_fields(buf: &[u8]) -> FieldIter<'_> {
    FieldIter {
        buf,
        offset: 0,
        done: false,
    }
}

pub struct FieldIter<'a> {
    buf: &'a [u8],
    offset: usize,
    done: bool,
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = Result<ProtoField<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.offset >= self.buf.len() {
            return None;
        }
        let (tag, next) = match decode_varint(self.buf, self.offset) {
            Ok(v) => v,
            Err(e) => {
                self.done = true;
                return Some(Err(e));
            }
        };
        self.offset = next;
        let num = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u32;
        match wire {
            0 => match decode_varint(self.buf, self.offset) {
                Ok((v, after)) => {
                    self.offset = after;
                    Some(Ok(ProtoField {
                        num,
                        wire,
                        value: FieldValue::Varint(v),
                    }))
                }
                Err(e) => {
                    self.done = true;
                    Some(Err(e))
                }
            },
            1 => {
                if self.offset + 8 > self.buf.len() {
                    self.done = true;
                    return None;
                }
                let value = FieldValue::Bytes(&self.buf[self.offset..self.offset + 8]);
                self.offset += 8;
                Some(Ok(ProtoField { num, wire, value }))
            }
            2 => match decode_varint(self.buf, self.offset) {
                Ok((n, after)) => {
                    self.offset = after;
                    let len = n as usize;
                    if self.offset + len > self.buf.len() {
                        self.done = true;
                        return None;
                    }
                    let value = FieldValue::Bytes(&self.buf[self.offset..self.offset + len]);
                    self.offset += len;
                    Some(Ok(ProtoField { num, wire, value }))
                }
                Err(e) => {
                    self.done = true;
                    Some(Err(e))
                }
            },
            5 => {
                if self.offset + 4 > self.buf.len() {
                    self.done = true;
                    return None;
                }
                let value = FieldValue::Bytes(&self.buf[self.offset..self.offset + 4]);
                self.offset += 4;
                Some(Ok(ProtoField { num, wire, value }))
            }
            _ => {
                self.done = true;
                None
            }
        }
    }
}

/// gzip 圧縮（TS `zlib.gzipSync` 相当 — deflate 出力 byte は
/// miniz_oxide 由来で zlib と一致しないことがあるが、wire contract は
/// 「valid gzip stream」であり復号互換）。
pub fn gzip(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    // write/finish が失敗するのは内部 buffer のみなので現実には到達しない。
    enc.write_all(data).expect("gzip write to Vec cannot fail");
    enc.finish().expect("gzip finish to Vec cannot fail")
}

/// gzip 伸長（TS `zlib.gunzipSync` 相当）。
pub fn gunzip(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut dec = flate2::read::GzDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| DecodeError::Gzip(e.to_string()))?;
    Ok(out)
}

/// `frameConnectStream` — `flags(1B) | len(4B BE) | payload`。
/// `compress` で flag `0x01` + gzip。
pub fn frame_connect_stream(body: &[u8], compress: bool) -> Vec<u8> {
    let (payload, flags) = if compress {
        (gzip(body), 0x01u8)
    } else {
        (body.to_vec(), 0x00u8)
    };
    let mut out = Vec::with_capacity(5 + payload.len());
    out.push(flags);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
    out
}
