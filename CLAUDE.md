# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`net` is a macOS CLI tool that displays network hardware ports with their IP addresses, link speed, and MAC addresses in a colorized table, sorted by network service order.

## Commands (run from project root)

```bash
cargo build              # debug build
cargo build --release    # optimized build
cargo run                # build and run (shows active ports only)
cargo run -- --all-ports # show all ports including ones without IPs
cargo run -- --help
cargo clippy             # lint
cargo test               # run tests
```

The compiled binary lands at `target/debug/net` (debug) or `target/release/net` (release).

## Architecture

A small library crate plus a thin binary:

- **`src/main.rs`** — CLI entry point: parses flags with `clap`, calls the pipeline below, renders via `print_table()` (`tabled`, per-column colorization), and prints errors in `Display` form (`net: <error>`) with a nonzero exit code.
- **`src/lib.rs`** — re-exports `NetError`, `HardwarePort`, `HardwarePortList` from their modules.
- **`src/command.rs`** — `run_command()` (strict: nonzero exit is a `NetError` naming the command and including stderr) and `run_command_allow_failure()` (nonzero exit → empty output; used for per-device queries where failure is a normal state).
- **`src/hardware_port.rs`** — one port's identity and live state. `get_ipaddr()` runs `ipconfig getifaddr <device>`; `get_speed()` parses the `media` lines of `ifconfig <device>` via the pure `map_speed_string()`. `service_order` is hidden from the table via `#[tabled(skip)]`.
- **`src/hardware_port_list.rs`** — discovery and ordering. Data flow:
  1. **`HardwarePortList::new()`** — runs `networksetup -listallhardwareports`, parses with the pure `parse_hardware_ports()` regex, queries each port's live state.
  2. **`.in_service_order()`** — runs `networksetup -listnetworkserviceorder`, maps devices to preference indices via the pure `parse_service_order()`, then sorts.
  3. **`.filter_ports()`** — drops ports with empty `ip_address` unless `--all-ports` is passed.
- **`src/error.rs`** — `NetError` (`CommandFailed` / `ParseError`) with `Display` messages meant for end users.

Parsing is deliberately split into pure functions (`parse_hardware_ports`, `parse_service_order`, `map_speed_string`) so unit tests cover them without running commands; `tests/cli.rs` runs the compiled binary end-to-end (e.g. with an empty `PATH` to force command failure).

**Key deps:** `clap` (derive feature) for CLI parsing, `regex` for parsing `networksetup` output, `tabled` for the colored table.

## macOS-only

All data comes from macOS system commands (`networksetup`, `ipconfig`, `ifconfig`). This tool will not work on Linux or Windows.
