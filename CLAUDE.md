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

## Architecture (`src/main.rs`)

Everything lives in a single file. The data flow is:

1. **`HardwarePortList::new()`** — runs `networksetup -listallhardwareports`, parses output with a regex to build a `Vec<HardwarePort>`.
2. **`.in_service_order()`** — runs `networksetup -listnetworkserviceorder | grep Device` to get preferred order, assigns `service_order` to each port, then sorts.
3. **`.filter_ports()`** — drops ports with empty `ip_address` unless `--all-ports` is passed.
4. **`print_table()`** — renders via `tabled` with per-column colorization.

**`HardwarePort`** — derives `tabled::Tabled` for zero-boilerplate table rendering. `get_ipaddr()` calls `ipconfig getifaddr <device>`; `get_speed()` pipes `ifconfig <device>` through `grep media` and maps the result to a human-readable string. `service_order` is hidden from the table via `#[tabled(skip)]`.

**Key deps:** `clap` (derive feature) for CLI parsing, `regex` for parsing `networksetup` output, `tabled` for the colored table.

## macOS-only

All data comes from macOS system commands (`networksetup`, `ipconfig`, `ifconfig`). This tool will not work on Linux or Windows.
