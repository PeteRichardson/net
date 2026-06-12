//! Custom error type for `net`, unifying command and parse failures.

use std::fmt;

/// Errors that can occur while discovering or querying network hardware ports.
#[derive(Debug)]
pub enum NetError {
    /// An external command (e.g. `networksetup`, `ifconfig`, `ipconfig`)
    /// could not be run or returned an I/O error.
    CommandFailed(String),
    /// Output from an external command could not be parsed (e.g. invalid UTF-8).
    ParseError(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::CommandFailed(msg) => write!(f, "command failed: {msg}"),
            NetError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for NetError {}

impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        NetError::CommandFailed(e.to_string())
    }
}

impl From<std::str::Utf8Error> for NetError {
    fn from(e: std::str::Utf8Error) -> Self {
        NetError::ParseError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for NetError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        NetError::ParseError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_command_failed() {
        let err = NetError::CommandFailed("boom".to_string());
        assert_eq!(err.to_string(), "command failed: boom");
    }

    #[test]
    fn test_display_parse_error() {
        let err = NetError::ParseError("bad utf8".to_string());
        assert_eq!(err.to_string(), "parse error: bad utf8");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err: NetError = io_err.into();
        match err {
            NetError::CommandFailed(msg) => assert!(msg.contains("no such file")),
            _ => panic!("expected CommandFailed"),
        }
    }

    #[test]
    fn test_from_utf8_error() {
        let bytes = vec![0, 159, 146, 150];
        let utf8_err = std::str::from_utf8(&bytes).unwrap_err();
        let err: NetError = utf8_err.into();
        match err {
            NetError::ParseError(_) => {}
            _ => panic!("expected ParseError"),
        }
    }

    #[test]
    fn test_from_from_utf8_error() {
        let bytes = vec![0, 159, 146, 150];
        let from_utf8_err = String::from_utf8(bytes).unwrap_err();
        let err: NetError = from_utf8_err.into();
        match err {
            NetError::ParseError(_) => {}
            _ => panic!("expected ParseError"),
        }
    }
}
