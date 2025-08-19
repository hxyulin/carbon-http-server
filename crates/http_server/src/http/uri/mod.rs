use std::{
    net::{AddrParseError, Ipv4Addr, Ipv6Addr},
    str::{FromStr, Utf8Error},
    string::FromUtf8Error,
};

use bytes::Bytes;
use uhsapi::ascii::{AsAsciiStr, AsciiString, InvalidAsciiError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum MalformedUriError {
    #[error("invalid IPv6 address")]
    InvalidAddress(#[from] AddrParseError),
    #[error(transparent)]
    InvalidAscii(#[from] InvalidAsciiError),
}

#[derive(Debug, Clone)]
pub enum IpLiteral {
    Ipv6(Ipv6Addr),
    IpvFuture(IpvFuture),
}

impl IpLiteral {
    fn from_str(s: &str) -> Result<Option<Self>, MalformedUriError> {
        if !s.starts_with('[') || !s.ends_with(']') {
            return Ok(None);
        }
        let s = &s[1..s.len() - 1];
        Ok(Some(if s.starts_with('v') {
            // IpvFuture, for now just hardcode
            // TODO: Actually implement
            Self::IpvFuture(IpvFuture {
                version: 0,
                content: s.as_ascii_str()?.to_ascii_string(),
            })
        } else {
            Self::Ipv6(s.parse()?)
        }))
    }
}

#[derive(Debug, Clone)]
pub struct IpvFuture {
    version: u32,
    content: AsciiString,
}

#[derive(Debug, Clone)]
pub enum UriHost {
    IpLiteral(IpLiteral),
    Ipv4(Ipv4Addr),
    RegName(AsciiString),
}

impl FromStr for UriHost {
    type Err = MalformedUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(addr) = IpLiteral::from_str(s)? {
            return Ok(Self::IpLiteral(addr));
        }

        // This is a bit of hack, not sure if this is spec compliant
        if let Ok(ipv4) = Ipv4Addr::from_str(s) {
            return Ok(Self::Ipv4(ipv4));
        } else {
            return Ok(Self::RegName(s.as_ascii_str()?.to_ascii_string()));
        }
    }
}

pub type UriPort = u16;

const HEX_CHARS_UPPER: &[u8] = b"0123456789ABCDEF";

fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

pub fn url_encode(input: &[u8]) -> String {
    let mut encoded = Vec::with_capacity(input.len() * 3); // Max 3 bytes per char (e.g., %FF)

    for &byte in input {
        if is_unreserved(byte) {
            encoded.push(byte);
        } else {
            encoded.push(b'%');
            encoded.push(HEX_CHARS_UPPER[(byte >> 4) as usize]); // Higher nibble
            encoded.push(HEX_CHARS_UPPER[(byte & 0xF) as usize]); // Lower nibble
        }
    }

    // This conversion should succeed because we only push ASCII characters or valid percent-encodings.
    // If the input was already valid UTF-8, and we only percent-encode certain bytes,
    // the output will still be valid UTF-8 ASCII for the percent-encoded parts.
    String::from_utf8(encoded).expect("URL encoded string should always be valid ASCII")
}

#[derive(Debug, thiserror::Error)]
pub enum UrlDecodeError {
    #[error("malformed encoding")]
    MalformedEncoding, // e.g., `%G1`, `%A`
    #[error(transparent)]
    InvalidUtf8(#[from] FromUtf8Error), // e.g., `%FF` if expecting String output
}

pub fn url_decode(input: &[u8]) -> Result<String, UrlDecodeError> {
    let mut decoded = Vec::with_capacity(input.len()); // Can be smaller or equal

    let mut i = 0;
    while i < input.len() {
        match input[i] {
            b'%' => {
                // Need 2 more bytes for hex digits
                if i + 2 >= input.len() {
                    return Err(UrlDecodeError::MalformedEncoding);
                }
                let hex_slice = &input[i + 1..i + 3];
                let byte_val = parse_hex_byte(hex_slice)?;
                decoded.push(byte_val);
                i += 3; // Advance past %HH
            }
            _ => {
                // Not percent-encoded, just append
                decoded.push(input[i]);
                i += 1;
            }
        }
    }

    // Finally, try to convert the decoded bytes to a String
    Ok(String::from_utf8(decoded)?)
}

fn parse_hex_byte(hex_slice: &[u8]) -> Result<u8, UrlDecodeError> {
    if hex_slice.len() != 2 {
        return Err(UrlDecodeError::MalformedEncoding);
    }
    let high = hex_to_digit(hex_slice[0])?;
    let low = hex_to_digit(hex_slice[1])?;
    Ok((high << 4) | low)
}

fn hex_to_digit(c: u8) -> Result<u8, UrlDecodeError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(UrlDecodeError::MalformedEncoding), // Invalid hex character
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_host_valid_ipv6() {
        let host: UriHost = "[::1]".parse().unwrap();
        assert!(matches!(
            host,
            UriHost::IpLiteral(IpLiteral::Ipv6(Ipv6Addr::LOCALHOST))
        ))
    }

    #[test]
    fn test_uri_host_valid_ipvfuture() {
        let host: UriHost = "[v5.123]".parse().unwrap();
        assert!(matches!(host, UriHost::IpLiteral(IpLiteral::IpvFuture(_))))
    }

    #[test]
    fn test_uri_host_invalid_ipv6() {
        let host: Result<UriHost, _> = "[1234::gggg]".parse();
        assert!(matches!(host, Err(MalformedUriError::InvalidAddress(_))))
    }

    #[test]
    fn test_urlencode_basic() {
        assert_eq!(url_encode(b"hello world"), "hello%20world");
        assert_eq!(url_encode(b"foo/bar"), "foo%2Fbar"); // Slash is reserved, encoded by default
        assert_eq!(url_encode(b"~_.-"), "~_.-"); // Unreserved
        assert_eq!(url_encode(b""), "");
        assert_eq!(url_encode(b"123"), "123");
    }

    #[test]
    fn test_urlencode_non_ascii() {
        // 'é' in UTF-8 is C3 A9
        assert_eq!(url_encode("é".as_bytes()), "%C3%A9");
        // Byte 0xFF (invalid UTF-8 as a single byte)
        assert_eq!(url_encode(&[0xFF]), "%FF");
    }

    #[test]
    fn test_urlencode_reserved_chars() {
        // Some common reserved characters
        assert_eq!(url_encode(b"&=+!@$#"), "%26%3D%2B%21%40%24%23");
        assert_eq!(url_encode(b"{}[]"), "%7B%7D%5B%5D");
    }

    #[test]
    fn test_urldecode_basic() {
        assert_eq!(url_decode(b"hello%20world").unwrap(), "hello world");
        assert_eq!(url_decode(b"foo%2Fbar").unwrap(), "foo/bar");
        assert_eq!(url_decode(b"~_.-").unwrap(), "~_.-");
        assert_eq!(url_decode(b"").unwrap(), "");
        assert_eq!(url_decode(b"123").unwrap(), "123");
    }

    #[test]
    fn test_urldecode_non_ascii() {
        assert_eq!(url_decode(b"%C3%A9").unwrap(), "é");
        // Test an invalid UTF-8 sequence, expect error
        assert!(url_decode(b"%FF").is_err());
    }

    #[test]
    fn test_urldecode_malformed() {
        assert!(url_decode(b"%").is_err());
        assert!(url_decode(b"%A").is_err());
        assert!(url_decode(b"%GG").is_err());
        assert!(url_decode(b"foo%").is_err());
        assert!(url_decode(b"foo%A").is_err());
        assert!(url_decode(b"foo%G1").is_err());
    }

    #[test]
    fn test_urldecode_with_pluses_not_spaces() {
        // Standard RFC 3986 decoding doesn't convert + to space.
        // Only x-www-form-urlencoded does.
        assert_eq!(url_decode(b"a+b").unwrap(), "a+b");
    }
}
