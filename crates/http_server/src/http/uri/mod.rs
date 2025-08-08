use std::{
    net::{AddrParseError, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

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
}
