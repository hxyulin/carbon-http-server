use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpVersion {
    pub major: u8,
    pub minor: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseHttpVersionError;

impl std::fmt::Display for ParseHttpVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid HTTP version")
    }
}

impl std::error::Error for ParseHttpVersionError {}

impl FromStr for HttpVersion {
    type Err = ParseHttpVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix("HTTP/") {
            let mut parts = s.splitn(2, '.');
            let major = parts
                .next()
                .ok_or(ParseHttpVersionError)?
                .parse::<u8>()
                .map_err(|_| ParseHttpVersionError)?;
            let minor = parts
                .next()
                .ok_or(ParseHttpVersionError)?
                .parse::<u8>()
                .map_err(|_| ParseHttpVersionError)?;
            Ok(HttpVersion { major, minor })
        } else {
            Err(ParseHttpVersionError)
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP/{}.{}", self.major, self.minor)
    }
}

