//! End-to-end tests that run the compiled `net` binary.

use std::process::Command;

/// `--help` must describe what the tool does, not leak an internal doc
/// comment about the `Config` struct.
#[test]
fn help_describes_the_tool() {
    let out = Command::new(env!("CARGO_BIN_EXE_net"))
        .arg("--help")
        .output()
        .expect("failed to run net binary");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Display macOS network hardware ports"),
        "help should describe the tool, got: {stdout}"
    );
    assert!(
        !stdout.contains("Command-line options"),
        "help should not leak the struct doc comment, got: {stdout}"
    );
}

/// When stdout is not a terminal (here: a pipe), the table must be plain
/// text — no ANSI escape codes to pollute `net | grep ...` or `net > file`.
#[test]
fn piped_output_has_no_ansi_escapes() {
    let out = Command::new(env!("CARGO_BIN_EXE_net"))
        .output()
        .expect("failed to run net binary");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains('\x1b'),
        "piped output should have no ANSI escapes, got: {stdout:?}"
    );
}

/// With an empty PATH, `networksetup` cannot be found: the tool must exit
/// nonzero and report the failure in its user-facing `Display` form
/// (`net: command failed: ...`), not as a `Debug` dump.
#[test]
fn error_is_printed_via_display_with_nonzero_exit() {
    let out = Command::new(env!("CARGO_BIN_EXE_net"))
        .env("PATH", "")
        .output()
        .expect("failed to run net binary");

    assert!(!out.status.success(), "expected nonzero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.starts_with("net: "),
        "stderr should use the Display format, got: {stderr}"
    );
    assert!(
        stderr.contains("networksetup"),
        "stderr should name the failing command, got: {stderr}"
    );
}
