//! Shared helpers for running external commands with consistent error handling.

use crate::error::NetError;
use std::process::{Command, Output};

/// Spawn `cmd` with `args` and collect its output, naming the command in the
/// error if it cannot be run at all.
fn spawn_and_collect(cmd: &str, args: &[&str]) -> Result<Output, NetError> {
    Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| NetError::CommandFailed(format!("{cmd}: {e}")))
}

/// Run `cmd` with `args` and return its stdout as a string.
///
/// # Errors
/// Returns [`NetError::CommandFailed`] naming the command if it cannot be
/// spawned or exits nonzero; the error includes the command's stderr.
pub(crate) fn run_command(cmd: &str, args: &[&str]) -> Result<String, NetError> {
    let output = spawn_and_collect(cmd, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetError::CommandFailed(format!(
            "{cmd} {}: exited with {} ({})",
            args.join(" "),
            output.status,
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Run `cmd` with `args`, tolerating a nonzero exit.
///
/// A nonzero exit yields `Ok(String::new())` — some queries (e.g.
/// `ipconfig getifaddr` on an interface with no address, `ifconfig` on an
/// irrelevant device) exit nonzero as a normal, expected outcome. Failure to
/// spawn the command at all is still an error.
///
/// # Errors
/// Returns [`NetError::CommandFailed`] naming the command if it cannot be spawned.
pub(crate) fn run_command_allow_failure(cmd: &str, args: &[&str]) -> Result<String, NetError> {
    let output = spawn_and_collect(cmd, args)?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_returns_stdout_on_success() {
        let out = run_command("echo", &["hello"]).unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn test_run_command_errors_on_nonzero_exit_and_names_command() {
        let err = run_command("false", &[]).unwrap_err();
        assert!(
            err.to_string().contains("false"),
            "error should name the command: {err}"
        );
    }

    #[test]
    fn test_run_command_includes_stderr_in_error() {
        let err = run_command("sh", &["-c", "echo oops >&2; exit 3"]).unwrap_err();
        assert!(
            err.to_string().contains("oops"),
            "error should include stderr: {err}"
        );
    }

    #[test]
    fn test_run_command_errors_when_binary_missing() {
        let err = run_command("definitely-not-a-real-command-xyz", &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("definitely-not-a-real-command-xyz"),
            "error should name the missing command: {err}"
        );
    }

    #[test]
    fn test_allow_failure_returns_stdout_on_success() {
        let out = run_command_allow_failure("echo", &["hi"]).unwrap();
        assert_eq!(out, "hi\n");
    }

    #[test]
    fn test_allow_failure_returns_empty_on_nonzero_exit() {
        let out = run_command_allow_failure("false", &[]).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_allow_failure_still_errors_when_binary_missing() {
        assert!(run_command_allow_failure("definitely-not-a-real-command-xyz", &[]).is_err());
    }
}
