---
git_sha: c706481
generated_at: 2026-07-10
scope: whole repo
---

# Project Review: `net`

## Executive summary

1. **External command exit statuses are never checked.** If `networksetup` fails, the tool silently prints an empty table instead of an error (`src/hardware_port_list.rs:58`). This is the single most impactful fix available.
2. **`CLAUDE.md` is badly stale** — it describes a single-file `src/main.rs` architecture and grep pipelines that were refactored away weeks ago. Every future Claude session starts with a wrong mental model.
3. **Errors reach the user as Rust `Debug` output** (`Error: CommandFailed("...")`) because `main` returns `Result<(), NetError>`; the carefully written `Display` impl in `src/error.rs` is never used.
4. **ANSI color codes are emitted unconditionally** — `net | grep en0` or `net > file` gets escape codes. No TTY detection, no `NO_COLOR` support.
5. **`--help`'s about line is a leaked doc comment**: users see "Command-line options for `net`" instead of what the tool does.
6. **`NetError::CommandFailed` doesn't say which command failed** — the blanket `From<io::Error>` erases context from all four call sites.
7. **No CI.** Tests and clippy run only when someone remembers; a macOS GitHub Actions workflow is ~20 lines.
8. A handful of `IDIOM`-tagged items (`&String` params, `contains_key` + index double-lookup, `&mut *self.ports`) — cheap fixes with language-fluency value.
9. Test coverage of pure logic is good (15 passing tests); every path that actually runs a command has zero coverage and no seam for injecting fake output.
10. The June 2026 tech-debt analysis is now ~70% resolved (monolith split, `NetError`, process reaping all landed) — it should be archived or refreshed.

## Architectural mental model

`net` is a 529-LOC macOS-only CLI, structured as a small library crate (`src/lib.rs` exporting `NetError`, `HardwarePort`, `HardwarePortList`) plus a thin binary (`src/main.rs`, 48 lines: parse flags → build list → sort → filter → render). Data flows through a builder-style pipeline: `HardwarePortList::new()` shells out to `networksetup -listallhardwareports` and, per port, `ipconfig getifaddr` + `ifconfig` (via `HardwarePort::query_network_state`); `.in_service_order()` shells out to `networksetup -listnetworkserviceorder` and sorts; `.filter_ports()` drops IP-less ports unless `--all-ports`; `print_table` renders with `tabled`. Parsing is deliberately extracted into pure functions (`parse_hardware_ports`, `parse_service_order`, `map_speed_string`) that carry all the unit tests, while the `Command`-running wrappers around them are untested.

**This model contradicts the repo's own `CLAUDE.md`**, which still describes a single-file `src/main.rs` with grep pipelines. The code is right; the doc is wrong (finding DOC-1). The git history shows a deliberate, well-sequenced hardening campaign (split → pure-function extraction → `NetError` → error propagation), so most of what remains is the tail of that effort, not neglect.

## Findings table

> 22 findings. The skill's 30–80 guideline assumes a larger repo; at 529 LOC, padding to 30 would be noise.

| ID | Category | File:Line | Severity | Effort | Description | Recommendation |
|----|----------|-----------|----------|--------|-------------|----------------|
| ERR-1 | error-handling | src/hardware_port_list.rs:58 | **High** | S | `output.status` is never checked on any of the four `Command` invocations (`hardware_port_list.rs:58`, `:87`; `hardware_port.rs:60`, `:74`). A failing `networksetup` produces empty stdout → the tool prints an empty table with exit code 0. | Add a shared `run_command(cmd, args) -> Result<String, NetError>` that checks `status.success()` and includes stderr in the error. `ipconfig getifaddr` is the exception — nonzero exit means "no IP", a normal state (see ERR-4). |
| DOC-1 | doc-drift | CLAUDE.md:9 | **High** | S | "Everything lives in a single file" / "Architecture (`src/main.rs`)" — false since commit b487b9a. Also claims `get_speed` "pipes through grep" and service order uses `\| grep Device`; grep was removed in 4fbf117. Misleads every AI-assisted session on this repo. | Rewrite the Architecture section: lib crate with `error.rs`, `hardware_port.rs`, `hardware_port_list.rs`; pure parse functions; no shell pipelines. |
| ERR-2 | error-handling | src/main.rs:40 | Medium | S | `main() -> Result<(), NetError>` prints errors via `Debug` (`Error: CommandFailed("...")`). The `Display` impl at `src/error.rs:15` never runs. | Match on the result: `eprintln!("net: {e}")` and return `std::process::ExitCode::FAILURE`. |
| UX-1 | ux-cli | src/main.rs:10 | Medium | S | `--help` about line is the struct's doc comment: "Command-line options for `net`". Describes the code, not the tool. | Change the doc comment on `Config` to e.g. "Display macOS network hardware ports with IP address, link speed, and MAC address, in service order." |
| UX-2 | ux-cli | src/main.rs:36 | Medium | M | Color is applied unconditionally; piped/redirected output keeps ANSI escape codes, and `NO_COLOR` is ignored. Verified: `cargo run \| head` emits raw `\x1b[37m...`. | Gate `Colorization` on `std::io::stdout().is_terminal()` (std `IsTerminal`, no new dep) and skip when `NO_COLOR` is set. |
| ERR-3 | error-handling | src/error.rs:26 | Medium | M | `From<std::io::Error>` produces `CommandFailed("No such file or directory (os error 2)")` — no hint which of the four external commands failed. | Drop the blanket `From`; have `run_command` (ERR-1) build `CommandFailed(format!("{cmd}: {e}"))`. One helper fixes ERR-1 and ERR-3 together. |
| DEP-2 | dependency-config | (repo root) | Medium | S | No CI. `cargo test` / `clippy` / `audit` run only ad hoc; a regression on main would go unnoticed until the next manual run. | Add `.github/workflows/ci.yml` on `macos-latest`: `cargo test`, `cargo clippy -- -D warnings`, `cargo audit`. |
| TEST-1 | test-debt | src/hardware_port.rs:48 | Medium | M | Every function that spawns a `Command` (`new`, `query_network_state`, `get_ipaddr`, `get_speed`, `get_service_order`) has zero test coverage and no seam for fake output. Pure parsers are well tested; the I/O wrappers around them are not. | The `run_command` helper from ERR-1 becomes the seam: the wrappers reduce to `parse_x(run_command(...)?)`, thin enough that testing the parsers suffices — plus one `#[ignore]`d smoke test that runs the real binary. |
| TYPE-1 | type-contract | src/hardware_port.rs:18 | Medium | M | `ip_address: String` uses `""` as a sentinel for "no IP"; the sentinel drives filtering (`hardware_port_list.rs:123`) and speed logic (`hardware_port.rs:107`). The states "no address" and "address present" are not distinguishable in the type. | `Option<String>` with `#[tabled(display_with = ...)]` (or a small newtype) makes the states explicit. Do this opportunistically — it touches several sites for modest gain. |
| UX-3 | ux-cli | src/main.rs:46 | Low | S | With no active ports (e.g. all network down), the tool prints an empty bordered box — headers only, no guidance. | When the filtered list is empty and `--all-ports` wasn't passed, print a hint to stderr: "no ports with IP addresses; use --all-ports to see all". |
| IDIOM-1 | IDIOM | src/hardware_port.rs:58, :73 | Medium (floor; true: Low) | S | `fn get_ipaddr(device: &String)` and `get_speed(device: &String, ...)` — taking `&String` instead of `&str` is the classic Rust interview flag; forces callers to hold a `String` and adds a level of indirection. | Change both to `&str`; call sites already pass `&self.device`, which derefs cleanly. |
| IDIOM-2 | IDIOM | src/hardware_port_list.rs:102 | Medium (floor; true: Low) | S | `contains_key` followed by index `services_in_order[&port.device]` — double hash lookup plus a panicking indexing op guarded by the check. | `port.service_order = services_in_order.get(&port.device).copied().unwrap_or(usize::MAX);` — one lookup, no panic path, deletes the if/else. |
| IDIOM-3 | IDIOM | src/hardware_port_list.rs:101 | Medium (floor; true: Low) | S | `for port in &mut *self.ports` — the explicit reborrow `&mut *` is unnecessary noise. | `for port in &mut self.ports`. |
| IDIOM-4 | IDIOM | src/hardware_port_list.rs:91 | Medium (floor; true: Low) | S | Lines are filtered, `collect`ed, and `join("\n")`ed — then `parse_service_order` immediately re-splits with `.lines()`. Build-a-string-to-split-a-string. | Pass the filtered iterator (or the raw stdout) into `parse_service_order` and do the `contains("Device")` filter there, next to the enumerate. |
| IDIOM-5 | IDIOM | src/hardware_port_list.rs:118 | Medium (floor; true: Low) | S | `filter_ports(active_only: bool)` is a boolean flag whose call site needs a comment to decode (`main.rs:45`: `filter_ports(!config.all_ports)` + trailing comment). Bool params that require negation at the call site are a readability smell. | Either branch in `main` (`if !config.all_ports { list = list.active_only(); }`) or take the flag un-negated: `filter_ports(show_all: bool)`. |
| CONS-1 | consistency | src/hardware_port.rs:64 | Low | S | Three UTF-8 decode styles across four sites: `String::from_utf8` (`hardware_port_list.rs:61`), `str::from_utf8` (`:90`, `hardware_port.rs:78`), `from_utf8_lossy` (`hardware_port.rs:64`). Output of the same class of external command is treated as fallible in three places and infallible in one. | Pick one policy. `from_utf8_lossy` everywhere is defensible for command output and would delete the two `From<Utf8Error>` impls in `error.rs`. |
| ERR-4 | error-handling | src/hardware_port.rs:58 | Low | S | `get_ipaddr` treats `ipconfig` failure and "no IP assigned" identically (both yield `""`). Today that's accidental — nonzero exit is simply never observed. | When ERR-1 lands, make this explicit: nonzero exit from `ipconfig getifaddr` → `Ok(String::new())` with a comment, so the conflation is a documented decision. |
| DOC-3 | doc-drift | src/hardware_port_list.rs:57, :77 | Low | S | Public `Result`-returning functions lack `# Errors` doc sections (clippy pedantic `missing_errors_doc`). | Add one-line `# Errors` sections; also fix `uninlined_format_args` at `main.rs:36` (`println!("{table}")`) and consider `#[must_use]` on `filter_ports`. |
| DOC-2 | doc-drift | README.md:10 | Low | S | No build/install instructions (it's a Rust project — `cargo install --path .` is one line); all three screenshots have alt text "alt". | Add a Build/Install section and real alt text. |
| ARCH-1 | arch-decay | src/hardware_port.rs:13 | Low | S | `#[derive(Tabled, Default)]` — `Default` is never used anywhere (`HardwarePort::new` and the test helper construct fields explicitly). | Drop `Default` from the derive. |
| PERF-1 | performance | src/hardware_port_list.rs:64 | Low | M | Two subprocesses are spawned serially per port (`ipconfig` + `ifconfig`), plus two list commands — on a Mac with 12+ hardware ports that's ~26 sequential process launches and is the entire runtime of the tool. | Only if latency ever bothers you: `std::thread::scope` to query ports concurrently. Not worth it preemptively. |
| DEP-1 | dependency-config | Cargo.toml:9 | Low | S | `tabled 0.21` pulls `proc-macro-error2 2.0.1`, flagged unmaintained (RUSTSEC-2026-0173). Build-time only; also `tabled` itself is several minor versions behind (0.21 vs 0.26+). | `cargo update` for the minor bumps; try upgrading `tabled` and see if the advisory clears. No urgency. |

## Related tactical findings

No `/code-review` reports exist in `docs/reviews/` yet. The pre-existing `docs/technical-debt-analysis-2026-06-11.md` served as a prior: its major items (single-file monolith, `unwrap()` sprawl, zombie processes, missing `NetError`) are all **resolved** by the June refactoring campaign; its still-open items (unmaintained `proc-macro-error2`, missing CI, I/O test gap) are carried forward here as DEP-1, DEP-2, and TEST-1.

## Top 5 "if you fix nothing else, fix these"

### 1. ERR-1 + ERR-3: one `run_command` helper, exit-status checked, command name in errors

```rust
// src/command.rs (new, ~20 lines) — or in error.rs
pub(crate) fn run_command(cmd: &str, args: &[&str]) -> Result<String, NetError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| NetError::CommandFailed(format!("{cmd}: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetError::CommandFailed(format!(
            "{cmd} {}: {}", args.join(" "), stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

Then `HardwarePortList::new()` becomes `parse_hardware_ports(&run_command("networksetup", &["-listallhardwareports"])?)`, and similarly for the other three call sites. `get_ipaddr` keeps its own non-checking path (nonzero exit = no IP, documented). This also deletes the blanket `From<io::Error>` and both UTF-8 `From` impls, and creates the test seam TEST-1 wants.

### 2. ERR-2: show users the `Display` message, not `Debug`

```rust
fn main() -> std::process::ExitCode {
    let config = Config::parse();
    match HardwarePortList::new()
        .and_then(HardwarePortList::in_service_order)
        .map(|l| l.filter_ports(!config.all_ports))
    {
        Ok(ports) => { print_table(ports); std::process::ExitCode::SUCCESS }
        Err(e) => { eprintln!("net: {e}"); std::process::ExitCode::FAILURE }
    }
}
```

### 3. DOC-1: rewrite CLAUDE.md's Architecture section

Replace the "Everything lives in a single file" paragraph with the actual module layout (`lib.rs` re-exporting from `error.rs` / `hardware_port.rs` / `hardware_port_list.rs`), and delete the two grep-pipeline claims. Five minutes; pays off in every future session.

### 4. UX-1: fix the `--help` about line

```rust
/// Display macOS network hardware ports with IP address, link speed,
/// and MAC address, sorted by network service order.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Config { ... }
```

### 5. DEP-2: minimal CI

`.github/workflows/ci.yml` on `macos-latest`: checkout, `cargo test`, `cargo clippy --all-targets -- -D warnings`. Add `cargo audit` as a non-blocking step. (Note: `cargo test` runs fine on the runner since all command execution is behind functions the tests don't call.)

## Quick wins

- [ ] ERR-2 — print errors via `Display`, exit code 1 (S)
- [ ] UX-1 — real about-line in `--help` (S)
- [ ] DOC-1 — un-stale CLAUDE.md (S)
- [ ] IDIOM-1 — `&String` → `&str` ×2 (S)
- [ ] IDIOM-2 — `get().copied().unwrap_or(usize::MAX)` (S)
- [ ] IDIOM-3 — drop the `&mut *` reborrow (S)
- [ ] ARCH-1 — remove unused `Default` derive (S)
- [ ] DOC-3 — `# Errors` sections + `println!("{table}")` (S)
- [ ] DEP-1 — `cargo update`, try newer `tabled` (S)

## Things that look bad but are actually fine

- **`Regex::new(...).unwrap()` at `src/hardware_port_list.rs:24`** — the pattern is a compile-time-known literal; if it's invalid that's a programmer error and a panic is correct. `LazyLock` would be over-engineering for a function called exactly once per run.
- **`usize::MAX` as the "not in service order" sort key (`src/hardware_port_list.rs:106`)** — an `Option<usize>` would be more honest but complicates the sort (`None` sorts *first* by default). The sentinel is commented, local to one function, and never escapes. Deliberate and fine.
- **Two-phase construction (`HardwarePort::new` then `query_network_state`, `src/hardware_port.rs:35`/`:48`)** — looks like a half-initialized-object smell, but commit a707a57 shows it's a deliberate separation of identity (parseable, testable) from live state (I/O). It's what makes the parser tests possible.
- **`get_service_order` as a nested fn inside `in_service_order` (`src/hardware_port_list.rs:79`)** — unusual placement, but it's used exactly once and the testable part (`parse_service_order`) is already extracted to module level. Locality is a valid choice here.
- **Everything is `String`s** — for a display-only tool feeding `tabled`, parsing IPs into `IpAddr` or MACs into a structured type would add conversions with zero user-visible benefit. (The `Option`-vs-empty-sentinel part is still worth fixing — that's TYPE-1 — but "stringly typed" overall is right-sized.)
- **`.vscode/launch.json` tracked in git** — often flagged as personal config, but for a solo macOS-only project a shared debug config is more useful than dogma.
- **`pub(crate)` fields on `HardwarePort` instead of getters** — appropriate for a binary-first crate; getter ceremony would be pure noise.
- **Duplicated filter-lines-then-join pattern (`src/hardware_port.rs:79`, `src/hardware_port_list.rs:91`)** — two sites, below any reasonable abstraction threshold; IDIOM-4 fixes one of them as a side effect anyway.

## Open questions for the maintainer

1. **`docs/*` is gitignored** (`.gitignore:2`) except `docs/images/` — so `docs/specs/`, `docs/plans/`, the tech-debt analysis, and this review exist only on this machine. Intentional local-only working docs, or an accident of the images change in c706481? If intentional, fine; if not, you're one disk failure away from losing the project's decision history.
2. **The `auto` speed for Wi-Fi** — the TODO at `src/hardware_port.rs:108` (use `CWWiFiClient` transmit rate, requires location-services permission) — is that still on the roadmap, or is `auto` the accepted end state? Affects whether TYPE-1's speed field is worth restructuring.
3. **Is distribution planned** (Homebrew tap, GitHub releases)? That would promote DEP-2 (CI) and DOC-2 (install docs) from Medium/Low to top priority.
4. **`docs/technical-debt-analysis-2026-06-11.md`** is now mostly resolved and its cost/ROI projections don't reflect the current codebase. Archive it, or refresh it? (Its remaining live items are all carried in this review.)
