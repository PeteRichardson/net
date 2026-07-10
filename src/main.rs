//! `net`: list macOS network hardware ports with their IP, speed, and MAC address.

use clap::Parser;
use net::{HardwarePortList, NetError};
use std::io::IsTerminal;
use std::process::ExitCode;
use tabled::{
    Table,
    settings::{Alignment, Color, Style, object::Columns, themes::Colorization},
};

/// Display macOS network hardware ports with IP address, link speed, and
/// MAC address, sorted by network service order.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Config {
    /// show all hw ports, not just ones with ip addresses
    #[clap(long, short, action)]
    pub all_ports: bool,
}

/// Color the table only when stdout is a terminal and `NO_COLOR` is unset
/// or empty (https://no-color.org), so piped or redirected output stays
/// plain text.
fn use_color() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

/// Render `data` as a rounded-border table and print it to stdout,
/// colourised per column when [`use_color`] allows.
///
/// The speed column (index 3) is right-aligned because it contains
/// fixed-width numeric strings.
fn print_table(data: HardwarePortList) {
    let mut table = Table::new(data.ports);
    table
        .with(Style::rounded())
        .modify(Columns::new(3..4), Alignment::right());

    if use_color() {
        table.with(Colorization::columns([
            Color::FG_WHITE,
            Color::FG_YELLOW,
            Color::FG_GREEN,
            Color::FG_BRIGHT_BLUE,
            Color::FG_BRIGHT_MAGENTA,
        ]));
    }

    println!("{table}");
}

/// Collect, sort, and filter the hardware ports to display.
fn collect_ports(config: &Config) -> Result<HardwarePortList, NetError> {
    let mut list = HardwarePortList::new()?.in_service_order()?;
    if !config.all_ports {
        list = list.active_only();
    }
    Ok(list)
}

/// Return the hint to print when there is nothing to show: the port list is
/// empty because inactive ports were filtered out and `--all-ports` was not
/// passed. Returns `None` when the user already asked for all ports.
fn no_ports_hint(config: &Config, ports: &HardwarePortList) -> Option<&'static str> {
    (ports.ports.is_empty() && !config.all_ports)
        .then_some("no ports with IP addresses; use --all-ports to see all")
}

/// Entry point: parse CLI flags, collect and sort hardware ports, then display them.
fn main() -> ExitCode {
    let config = Config::parse();

    match collect_ports(&config) {
        Ok(ports) => {
            if let Some(hint) = no_ports_hint(&config, &ports) {
                eprintln!("net: {hint}");
            }
            print_table(ports);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("net: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(all_ports: bool) -> Config {
        Config { all_ports }
    }

    fn empty_list() -> HardwarePortList {
        HardwarePortList { ports: vec![] }
    }

    #[test]
    fn test_hint_shown_when_filtered_list_is_empty() {
        let hint = no_ports_hint(&config(false), &empty_list());
        assert_eq!(
            hint,
            Some("no ports with IP addresses; use --all-ports to see all")
        );
    }

    #[test]
    fn test_no_hint_with_all_ports_flag() {
        assert_eq!(no_ports_hint(&config(true), &empty_list()), None);
    }
}
