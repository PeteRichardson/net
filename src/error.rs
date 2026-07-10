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
}
