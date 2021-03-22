//!CDRS support traffic decompression as it is described in [Apache
//!Cassandra protocol](
//!https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v4.spec#L790)
//!
//!Before being used, client and server must agree on a compression algorithm to
//!use, which is done in the STARTUP message. As a consequence, a STARTUP message
//!must never be compressed.  However, once the STARTUP frame has been received
//!by the server, messages can be compressed (including the response to the STARTUP
//!request).

use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::result;

use lz4_compress as lz4;
use snap::raw::{Decoder, Encoder};

type Result<T> = result::Result<T, CompressionError>;

pub const LZ4: &str = "lz4";
pub const SNAPPY: &str = "snappy";

/// It's an error which may occur during encoding or decoding
/// frame body. As there are only two types of compressors it
/// contains two related enum options.
#[derive(Debug)]
pub enum CompressionError {
    /// Snappy error.
    Snappy(snap::Error),
    /// Lz4 error.
    Lz4(io::Error),
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CompressionError::Snappy(ref err) => write!(f, "Snappy Error: {:?}", err),
            CompressionError::Lz4(ref err) => write!(f, "Lz4 Error: {:?}", err),
        }
    }
}

impl Error for CompressionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            CompressionError::Snappy(ref err) => Some(err),
            CompressionError::Lz4(ref err) => Some(err),
        }
    }
}

/// Enum which represents a type of compression. Only non-startup frame's body can be compressed.
#[derive(Debug, PartialEq, Clone, Copy, Eq, Ord, PartialOrd)]
pub enum Compression {
    /// [lz4](https://code.google.com/p/lz4/) compression
    Lz4,
    /// [snappy](https://code.google.com/p/snappy/) compression
    Snappy,
    /// Non compression
    None,
}

impl Compression {
    /// It encodes `bytes` basing on type of `Compression`..
    ///
    /// # Examples
    ///
    /// ```
    ///    use cdrs_tokio::compression::Compression;
    ///
    ///   let snappy_compression = Compression::Snappy;
    ///   let bytes = String::from("Hello World").into_bytes().to_vec();
    ///   let encoded = snappy_compression.encode(bytes.clone()).unwrap();
    ///   assert_eq!(snappy_compression.decode(encoded).unwrap(), bytes);
    ///
    /// ```
    pub fn encode(&self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        match *self {
            Compression::Lz4 => Ok(Compression::encode_lz4(bytes)),
            Compression::Snappy => Compression::encode_snappy(bytes),
            Compression::None => Ok(bytes),
        }
    }

    /// It decodes `bytes` basing on type of compression.
    ///
    /// # Examples
    ///
    /// ```
    ///    use cdrs_tokio::compression::Compression;
    ///     let lz4_compression = Compression::Lz4;
    ///     let bytes = String::from("Hello World").into_bytes().to_vec();
    ///     let encoded = lz4_compression.encode(bytes.clone()).unwrap();
    ///     let len = encoded.len() as u8;
    ///     let mut input = vec![0, 0, 0, len];
    ///     input.extend_from_slice(encoded.as_slice());
    ///     assert_eq!(lz4_compression.decode(input).unwrap(), bytes);
    /// ```
    pub fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        match *self {
            Compression::Lz4 => Compression::decode_lz4(bytes),
            Compression::Snappy => Compression::decode_snappy(bytes),
            Compression::None => Ok(bytes),
        }
    }

    /// It transforms compression method into a `&str`.
    pub fn as_str(&self) -> Option<&'static str> {
        match *self {
            Compression::Lz4 => Some(LZ4),
            Compression::Snappy => Some(SNAPPY),
            Compression::None => None,
        }
    }

    fn encode_snappy(bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut encoder = Encoder::new();
        encoder
            .compress_vec(bytes.as_slice())
            .map_err(CompressionError::Snappy)
    }

    fn decode_snappy(bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut decoder = Decoder::new();
        decoder
            .decompress_vec(bytes.as_slice())
            .map_err(CompressionError::Snappy)
    }

    fn encode_lz4(bytes: Vec<u8>) -> Vec<u8> {
        lz4::compress(bytes.as_slice())
    }

    fn decode_lz4(bytes: Vec<u8>) -> Result<Vec<u8>> {
        // skip first 4 bytes in accordance to
        // https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v4.spec#L805
        lz4::decompress(&bytes[4..])
            .map_err(|error| CompressionError::Lz4(io::Error::new(io::ErrorKind::Other, error)))
    }
}

impl From<String> for Compression {
    /// It converts `String` into `Compression`. If string is neither `lz4` nor `snappy` then
    /// `Compression::None` will be returned
    fn from(compression_string: String) -> Compression {
        Compression::from(compression_string.as_str())
    }
}

impl<'a> From<&'a str> for Compression {
    /// It converts `str` into `Compression`. If string is neither `lz4` nor `snappy` then
    /// `Compression::None` will be returned
    fn from(compression_str: &'a str) -> Compression {
        match compression_str {
            LZ4 => Compression::Lz4,
            SNAPPY => Compression::Snappy,
            _ => Compression::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_compression_from_str() {
        let lz4 = "lz4";
        assert_eq!(Compression::from(lz4), Compression::Lz4);
        let snappy = "snappy";
        assert_eq!(Compression::from(snappy), Compression::Snappy);
        let none = "x";
        assert_eq!(Compression::from(none), Compression::None);
    }

    #[test]
    fn test_compression_from_string() {
        let lz4 = "lz4".to_string();
        assert_eq!(Compression::from(lz4), Compression::Lz4);
        let snappy = "snappy".to_string();
        assert_eq!(Compression::from(snappy), Compression::Snappy);
        let none = "x".to_string();
        assert_eq!(Compression::from(none), Compression::None);
    }

    #[test]
    fn test_compression_encode_snappy() {
        let snappy_compression = Compression::Snappy;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        snappy_compression
            .encode(bytes)
            .expect("Should work without exceptions");
    }

    #[test]
    fn test_compression_decode_snappy() {
        let snappy_compression = Compression::Snappy;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        let encoded = snappy_compression.encode(bytes.clone()).unwrap();
        assert_eq!(snappy_compression.decode(encoded).unwrap(), bytes);
    }

    #[test]
    fn test_compression_encode_lz4() {
        let snappy_compression = Compression::Lz4;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        snappy_compression
            .encode(bytes)
            .expect("Should work without exceptions");
    }

    #[test]
    fn test_compression_decode_lz4() {
        let lz4_compression = Compression::Lz4;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        let encoded = lz4_compression.encode(bytes.clone()).unwrap();
        let len = encoded.len() as u8;
        let mut input = vec![0, 0, 0, len];
        input.extend_from_slice(encoded.as_slice());
        assert_eq!(lz4_compression.decode(input).unwrap(), bytes);
    }

    #[test]
    fn test_compression_encode_none() {
        let none_compression = Compression::None;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        none_compression
            .encode(bytes)
            .expect("Should work without exceptions");
    }

    #[test]
    fn test_compression_decode_none() {
        let none_compression = Compression::None;
        let bytes = String::from("Hello World").into_bytes().to_vec();
        let encoded = none_compression.encode(bytes.clone()).unwrap();
        assert_eq!(none_compression.decode(encoded).unwrap(), bytes);
    }

    #[test]
    fn test_compression_encode_lz4_with_invalid_input() {
        let lz4_compression = Compression::Lz4;
        let bytes: Vec<u8> = vec![0x7f, 0x7f, 0x7f, 0x7f, 0x7f];
        let encoded = lz4_compression.encode(bytes).unwrap();
        let decode = lz4_compression.decode(encoded);
        assert_eq!(decode.is_err(), true);
    }

    #[test]
    fn test_compression_encode_snappy_with_non_utf8() {
        let snappy_compression = Compression::Snappy;
        let v = vec![0xff, 0xff];
        let encoded = snappy_compression
            .encode(v.clone())
            .expect("Should work without exceptions");
        assert_eq!(snappy_compression.decode(encoded).unwrap(), v);
    }
}
