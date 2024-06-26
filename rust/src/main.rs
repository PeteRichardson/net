use clap::Parser;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::{Command, Stdio};
use std::str;
use tabled::{
    settings::{object::Columns, themes::Colorization, Alignment, Color, Style},
    Table, Tabled,
};

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Config {
    /// show all hw ports, not just ones with ip addresses
    #[clap(long, short, action)]
    pub all_ports: bool,
}

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
    #[tabled(skip)]
    service_order: u8,
}

impl HardwarePort {
    fn new(name: String, device: String, mac_address: String) -> Self {
        let ip_address = HardwarePort::get_ipaddr(&device);
        let speed = HardwarePort::get_speed(&device, &ip_address);
        Self {
            name,
            ip_address,
            speed,
            device,
            mac_address,
            service_order: 0,
        }
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

    fn get_speed(device: &String, ip: &str) -> String {
        //ifconfig {device} | grep media
        let ifconfig_child = Command::new("ifconfig") // `ifconfig` command...
            .arg(device) // with argument `axww`...
            .stdout(Stdio::piped()) // of which we will pipe the output.
            .spawn() // Once configured, we actually spawn the command...
            .unwrap(); // and assert everything went right.
        let grep_child_one = Command::new("grep")
            .arg("media")
            .stdin(Stdio::from(ifconfig_child.stdout.unwrap())) // Pipe through.
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let output = grep_child_one.wait_with_output().unwrap();
        let mut result = str::from_utf8(&output.stdout).unwrap();
        if result.contains("10G") {
            result = "10GbE";
        } else if result.contains("1000") {
            result = "1GbE";
        } else if !ip.is_empty() && result.contains("auto") {
                result = "auto";
            } else {
                result = "";
            }
        result.trim().to_string()
    }
}

struct HardwarePortList {
    ports: Vec<HardwarePort>,
}

impl HardwarePortList {
    fn new() -> Self {
        let mut port_data: Vec<HardwarePort> = Vec::new();
        let ports = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()
            .unwrap();
        let stdout = String::from_utf8(ports.stdout).expect("bad stdout from networksetup command");

        let re =
            Regex::new(r"Hardware Port: (.*)\nDevice: (.*)\nEthernet Address: (.*)\n\n").unwrap();
        for caps in re.captures_iter(&stdout) {
            let portname = caps[1].to_string();
            let device: String = caps[2].to_string();
            let mac_address = caps[3].to_string();
            port_data.push(HardwarePort::new(portname, device, mac_address))
        }

        //HardwarePortList::sort_by_service_order(&mut port_data);
        Self { ports: port_data }
    }

    fn in_service_order(mut self) -> Self {
        fn get_service_order() -> HashMap<String, u8> {
            // Returns a hash mapping port names to service order
            // e.g.  "en7" -> 0, "en8" -> 1, "WiFi" -> 3
            // Used to sort ports for printing
            //
            // uses the shell command:
            //    networksetup -listnetworkserviceorder | grep Device
            //
            // which has sample output:
            //      (Hardware Port: Thunderbolt Ethernet Slot 1, Device: en7)
            //      (Hardware Port: Thunderbolt Ethernet Slot 0, Device: en8)
            //      (Hardware Port: Thunderbolt Bridge, Device: bridge0)
            //      (Hardware Port: Wi-Fi, Device: en0)
            let networksetup_child = Command::new("networksetup")
                .arg("-listnetworkserviceorder") // with argument `axww`...
                .stdout(Stdio::piped()) // of which we will pipe the output.
                .spawn() // Once configured, we actually spawn the command...
                .unwrap(); // and assert everything went right.
            let grep_child_one = Command::new("grep")
                .arg("Device")
                .stdin(Stdio::from(networksetup_child.stdout.unwrap())) // Pipe through.
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let output = grep_child_one.wait_with_output().unwrap();
            let result = str::from_utf8(&output.stdout).unwrap();

            //println!("{}", result);
            let mut service_order: HashMap<String, u8> = HashMap::new();
            for (i,line) in result.lines().enumerate() {
                // remove trailing ')'
                let mut device: &str = line
                    .strip_suffix(|_: char| true)
                    .expect("no ) at end of serviceorder line!");
                device = device
                    .split_ascii_whitespace()
                    .last()
                    .expect("Couldn't split on whitespace?");
                service_order.insert(device.to_string(), i.try_into().unwrap());
            }

            service_order
        }

        let services_in_order = get_service_order();
        for port in &mut *self.ports {
            if services_in_order.contains_key(&port.device) {
                port.service_order = services_in_order[&port.device];
            } else {
                port.service_order = 255;
            }
        }

        self.ports.sort_by_key(|d1| d1.service_order);
        self
    }

    fn filter_ports(self, active_only: bool) -> Self {
        if active_only {
            let ports = self
                .ports
                .into_iter()
                .filter(|p| !(p.ip_address).is_empty())
                .collect();
            Self { ports }
        } else {
            self
        }
    }
}

fn print_table(data: HardwarePortList) -> Result<(), Box<dyn Error>> {
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

    println!("{}", table.to_string());
    Ok(())
}

fn main() {
    let config = Config::parse();

    let hardware_ports = HardwarePortList::new()
        .in_service_order()
        .filter_ports(!config.all_ports); // filter to active ports only, unless -all-ports
    print_table(hardware_ports).expect("Failed to output table");
}
