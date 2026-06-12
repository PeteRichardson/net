//! `net`: list macOS network hardware ports with their IP, speed, and MAC address.

use clap::Parser;
use net::{HardwarePortList, NetError};
use tabled::{
    Table,
    settings::{Alignment, Color, Style, object::Columns, themes::Colorization},
};

/// Command-line options for `net`.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Config {
    /// show all hw ports, not just ones with ip addresses
    #[clap(long, short, action)]
    pub all_ports: bool,
}

/// Render `data` as a colourised, rounded-border table and print it to stdout.
///
/// Each column is assigned a distinct foreground colour; the speed column
/// (index 3) is right-aligned because it contains fixed-width numeric strings.
fn print_table(data: HardwarePortList) {
    let mut table = Table::new(data.ports);
    table
        .with(Style::rounded())
        .with(Colorization::columns([
            Color::FG_WHITE,
            Color::FG_YELLOW,
            Color::FG_GREEN,
            Color::FG_BRIGHT_BLUE,
            Color::FG_BRIGHT_MAGENTA,
        ]))
        .modify(Columns::new(3..4), Alignment::right());

    println!("{}", table);
}

/// Entry point: parse CLI flags, collect and sort hardware ports, then display them.
fn main() -> Result<(), NetError> {
    let config = Config::parse();

    let hardware_ports = HardwarePortList::new()?
        .in_service_order()?
        .filter_ports(!config.all_ports); // filter to active ports only, unless -all-ports
    print_table(hardware_ports);
    Ok(())
}
