// use std::process::Command;
use tabled::{
    settings::{object::Columns, themes::Colorization, Alignment, Color, Style},
    Table, Tabled,
};

#[derive(Tabled)]
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
    let port1 = HardwarePort {
        name: "Thunderbolt Ethernet Slot 0".to_string(),
        ip_address: "192.168.1.57".to_string(),
        device: "en7".to_string(),
        speed: String::from("10GbE"),
        mac_address: "00:30:93:10:47:a2".to_string(),
    };
    let port2 = HardwarePort {
        name: "Wi-Fi".to_string(),
        ip_address: "192.168.1.185".to_string(),
        device: "en0".to_string(),
        speed: String::from("1GbE"),
        mac_address: "60:3e:5f:70:c4:64".to_string(),
    };
    let port3 = HardwarePort {
        name: "Thunderbolt Bridge".to_string(),
        ip_address: "".to_string(),
        device: "bridge0".to_string(),
        speed: String::from("1GbE"),
        mac_address: "36:ec:0e:59:56:40".to_string(),
    };

    vec![port1, port2, port3]
}

fn main() {
    // let ports = Command::new("networksetup")
    //     .arg("-listallhardwareports")
    //     .output()
    //     .unwrap();
    // println!("{}", String::from_utf8(ports.stdout).unwrap());

    let mut table = Table::new(get_net_data());
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
