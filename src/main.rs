use regex::Regex;
use std::process::Command;
use tabled::{
    settings::{object::Columns, themes::Colorization, Alignment, Color, Style},
    Table, Tabled,
};

#[derive(Tabled, Default)]
#[tabled(rename_all = "PascalCase")]
struct HardwarePort {
    name: String,
    #[tabled(rename = "IP Address")]
    ip_address: String,
    device: String,
    speed: String,
    #[tabled(rename = "MAC Address")]
    mac_address: String,
}

fn get_net_data() -> Vec<HardwarePort> {
    let mut data: Vec<HardwarePort> = Vec::new();
    let ports = Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()
        .unwrap();
    let stdout = String::from_utf8(ports.stdout).expect("bad stdout from networksetup command");

    let re = Regex::new(r"Hardware Port: (.*)\nDevice: (.*)\nEthernet Address: (.*)\n\n").unwrap();
    for caps in re.captures_iter(&stdout) {
        data.push(HardwarePort {
            name: caps[1].to_string(),
            device: caps[2].to_string(),
            mac_address: caps[3].to_string(),
            ..Default::default()
        })
    }
    data
}

fn main() {
    let net_data = get_net_data();
    let mut table = Table::new(net_data);
    table.with(Style::rounded()).with(Colorization::columns([
        Color::FG_WHITE,
        Color::FG_YELLOW,
        Color::FG_GREEN,
        Color::FG_BRIGHT_BLUE,
        Color::FG_BRIGHT_MAGENTA,
    ]));
    table.modify(Columns::new(3..4), Alignment::right());

    println!("{}", table.to_string());
}
