use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAsciiError;

impl fmt::Display for InvalidAsciiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid ascii")
    }
}

impl std::error::Error for InvalidAsciiError {}

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AsciiString {
    bytes: Vec<u8>,
}

impl fmt::Debug for AsciiString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // SAFETY: Valid ascii is guaranteed to be valid UTF-8
        write!(f, "\"{}\"", self.as_str())
    }
}

impl fmt::Display for AsciiString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn bytes_are_ascii(bytes: &[u8]) -> Result<(), InvalidAsciiError> {
    bytes.iter().all(|&b| b < 0x80).ok_or(InvalidAsciiError)
}

impl AsciiString {

    pub fn from_str(s: &str) -> Result<AsciiString, InvalidAsciiError> {
        Self::from_ascii(s.as_bytes())
    }

    pub fn from_ascii(bytes: &[u8]) -> Result<AsciiString, InvalidAsciiError> {
        bytes_are_ascii(bytes)?;
        // SAFETY: We checked that all bytes are valid
        Ok(unsafe { Self::from_ascii_unchecked(bytes) })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<AsciiString, InvalidAsciiError> {
        bytes_are_ascii(&bytes)?;
        // SAFETY: We checked that all bytes are valid
        Ok(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    pub unsafe fn from_bytes_unchecked(bytes: Vec<u8>) -> AsciiString {
        Self { bytes }
    }

    pub unsafe fn from_ascii_unchecked(bytes: &[u8]) -> AsciiString {
        Self {
            bytes: Vec::from(bytes),
        }
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: valid ascii is valid UTF-8
        unsafe { std::str::from_utf8_unchecked(self.bytes.as_slice()) }
    }
}

#[repr(transparent)]
#[derive(PartialEq, Eq, Hash)]
pub struct AsciiStr([u8]);

impl fmt::Debug for AsciiStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // SAFETY: Valid ascii is guaranteed to be valid UTF-8
        write!(f, "\"{}\"", self.as_str())
    }
}

impl fmt::Display for AsciiStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsciiStr {
    pub fn from_str(s: &str) -> Result<&AsciiStr, InvalidAsciiError> {
        Self::from_ascii(s.as_bytes())
    }

    pub const unsafe fn from_str_unchecked(s: &str) -> &AsciiStr {
        unsafe { Self::from_ascii_unchecked(s.as_bytes()) }
    }

    pub fn from_ascii(bytes: &[u8]) -> Result<&AsciiStr, InvalidAsciiError> {
        bytes.iter().all(|&b| b < 0x80).ok_or(InvalidAsciiError)?;
        Ok(unsafe { Self::from_ascii_unchecked(bytes) })
    }

    pub const unsafe fn from_ascii_unchecked(bytes: &[u8]) -> &AsciiStr {
        unsafe { std::mem::transmute(bytes) }
    }

    pub fn to_ascii_string(&self) -> AsciiString {
        // SAFETY: we are in a valid AsciiStr, so it is valid ascii
        unsafe { AsciiString::from_ascii_unchecked(&self.0) }
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: valid ascii is valid UTF-8
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<str> for &'_ AsciiStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

pub trait AsAsciiStr {
    fn as_ascii_str(&self) -> Result<&AsciiStr, InvalidAsciiError>;
}

impl AsAsciiStr for &'_ str {
    fn as_ascii_str(&self) -> Result<&AsciiStr, InvalidAsciiError> {
        AsciiStr::from_str(self)
    }
}

impl AsAsciiStr for &'_ [u8] {
    fn as_ascii_str(&self) -> Result<&AsciiStr, InvalidAsciiError> {
        AsciiStr::from_ascii(self)
    }
}

pub trait ToAsciiString {
    fn to_ascii_string(&self) -> Result<AsciiString, InvalidAsciiError>;
}

pub trait IntoAsciiString {
    fn into_ascii_string(self) -> Result<AsciiString, InvalidAsciiError>;
}

impl IntoAsciiString for String {
    fn into_ascii_string(self) -> Result<AsciiString, InvalidAsciiError> {
        AsciiString::from_bytes(self.into_bytes())
    }
}
