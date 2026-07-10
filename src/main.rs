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
    Ok(HardwarePortList::new()?
        .in_service_order()?
        .filter_ports(!config.all_ports)) // filter to active ports only, unless --all-ports
}

/// Entry point: parse CLI flags, collect and sort hardware ports, then display them.
fn main() -> ExitCode {
    let config = Config::parse();

    match collect_ports(&config) {
        Ok(ports) => {
            print_table(ports);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("net: {e}");
            ExitCode::FAILURE
        }
    }
}
