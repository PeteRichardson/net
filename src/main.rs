use regex::Regex;
use std::process::{Command, Stdio};
use std::str;
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

fn get_ipaddr(device: &String) -> String {
    //ipconfig getifaddr {device}
    let ports = Command::new("ipconfig")
        .arg("getifaddr")
        .arg(device)
        .output()
        .unwrap();

    let stdout =
        String::from_utf8(ports.stdout).expect("bad stdout from ipconfig getifaddr command");
    stdout.trim().to_string()
}

fn get_speed(device: &String, ip: &String) -> String {
    //ifconfig {device} | grep media
    let ps_child = Command::new("ifconfig") // `ps` command...
        .arg(device) // with argument `axww`...
        .stdout(Stdio::piped()) // of which we will pipe the output.
        .spawn() // Once configured, we actually spawn the command...
        .unwrap(); // and assert everything went right.
    let grep_child_one = Command::new("grep")
        .arg("media")
        .stdin(Stdio::from(ps_child.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let output = grep_child_one.wait_with_output().unwrap();
    let mut result = str::from_utf8(&output.stdout).unwrap();

    if result.contains("10G") {
        result = "10GbE";
    } else if result.contains("1000") {
        result = "1GbE";
    } else {
        result = "";
        if !ip.is_empty() {
            result = "auto?";
        }
    }
    result.trim().to_string()
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
        let portname = caps[1].to_string();
        let device: String = caps[2].to_string();
        let ip = get_ipaddr(&device);
        let speed = get_speed(&device, &ip);
        data.push(HardwarePort {
            name: portname,
            ip_address: ip,
            device: device,
            speed: speed,
            mac_address: caps[3].to_string(),
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
