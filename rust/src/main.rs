use clap::Parser;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::{Command, Stdio};
use std::str;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Color, Style, object::Columns, themes::Colorization},
};

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Config {
    /// show all hw ports, not just ones with ip addresses
    #[clap(long, short, action)]
    pub all_ports: bool,
}

/// Represents a macOS hardware network port and its current network state.
///
/// Fields are populated from `networksetup -listallhardwareports` and
/// supplemental `ipconfig` / `ifconfig` queries. `service_order` is excluded
/// from table output and is used only to sort ports for display.
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
    // Not rendered in the table; populated by `in_service_order()` and
    // used as the sort key so ports are listed in network preference order.
    #[tabled(skip)]
    service_order: usize,
}

impl HardwarePort {
    /// Construct a `HardwarePort` from the identifying fields returned by
    /// `networksetup -listallhardwareports`, querying for IP address and
    /// link speed as part of initialization.
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

    /// Return the IPv4 address currently assigned to `device`, or an empty
    /// string if the interface has no address.
    ///
    /// Delegates to `ipconfig getifaddr <device>`.
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

    /// Return a human-readable link-speed string for `device` (e.g. `"1GbE"`,
    /// `"100Mbps"`), or an empty string if speed cannot be determined.
    ///
    /// Parses the `media` line from `ifconfig <device>`. When the interface
    /// reports `auto` and an IP is present, returns `"auto"` because the
    /// negotiated rate is not accessible without location-services permission.
    fn get_speed(device: &String, ip: &str) -> String {
        //ifconfig {device} | grep media
        let ifconfig_child = Command::new("ifconfig")
            .arg(device)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let grep_child_one = Command::new("grep")
            .arg("media")
            .stdin(Stdio::from(ifconfig_child.stdout.unwrap())) // Pipe through.
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let output = grep_child_one.wait_with_output().unwrap();
        let result = str::from_utf8(&output.stdout).unwrap();
        map_speed_string(result, ip).to_string()
    }
}

/// An ordered collection of hardware network ports discovered on this machine.
struct HardwarePortList {
    ports: Vec<HardwarePort>,
}

impl HardwarePortList {
    /// Discover all hardware network ports by running `networksetup -listallhardwareports`
    /// and construct a `HardwarePort` for each one.
    ///
    /// The returned list is in the arbitrary order that `networksetup` emits, not
    /// service-preference order; call `in_service_order()` to sort before display.
    fn new() -> Self {
        let mut port_data: Vec<HardwarePort> = Vec::new();
        let ports = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()
            .unwrap();
        let stdout = String::from_utf8(ports.stdout).expect("bad stdout from networksetup command");

        // Each port block in the output is three lines followed by a blank line.
        // The \r? handles both LF and CRLF line endings defensively.
        let re = Regex::new(
            r"Hardware Port: ([^\r\n]*)\r?\nDevice: ([^\r\n]*)\r?\nEthernet Address: ([^\r\n]*)\r?\n\r?\n",
        )
        .unwrap();
        for caps in re.captures_iter(&stdout) {
            let portname = caps[1].to_string();
            let device: String = caps[2].to_string();
            let mac_address = caps[3].to_string();
            port_data.push(HardwarePort::new(portname, device, mac_address))
        }

        //HardwarePortList::sort_by_service_order(&mut port_data);
        Self { ports: port_data }
    }

    /// Re-order ports to match the network service priority set in System Settings.
    ///
    /// Ports absent from the service order (e.g. virtual or inactive interfaces)
    /// are placed at the end by assigning them `usize::MAX` as their sort key.
    fn in_service_order(mut self) -> Self {
        fn get_service_order() -> HashMap<String, usize> {
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
                .arg("-listnetworkserviceorder")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let grep_child_one = Command::new("grep")
                .arg("Device")
                .stdin(Stdio::from(networksetup_child.stdout.unwrap())) // Pipe through.
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let output = grep_child_one.wait_with_output().unwrap();
            let result = str::from_utf8(&output.stdout).unwrap();

            //println!("{}", result);
            let mut service_order: HashMap<String, usize> = HashMap::new();
            for (i, line) in result.lines().enumerate() {
                // remove trailing ')'
                let mut device: &str = line
                    .strip_suffix(')')
                    .expect("no ) at end of serviceorder line!");
                device = device
                    .split_ascii_whitespace()
                    .last()
                    .expect("Couldn't split on whitespace?");
                service_order.insert(device.to_string(), i);
            }

            service_order
        }

        let services_in_order = get_service_order();
        for port in &mut *self.ports {
            if services_in_order.contains_key(&port.device) {
                port.service_order = services_in_order[&port.device].clone();
            } else {
                // Ports not present in the service order list sort to the bottom.
                port.service_order = usize::MAX;
            }
        }

        self.ports.sort_by_key(|d1| d1.service_order);
        self
    }

    /// Optionally remove ports that have no IP address assigned.
    ///
    /// When `active_only` is `true`, only ports with a non-empty `ip_address`
    /// are retained. When `false`, all ports are returned unchanged.
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

/// Render `data` as a colourised, rounded-border table and print it to stdout.
///
/// Each column is assigned a distinct foreground colour; the speed column
/// (index 3) is right-aligned because it contains fixed-width numeric strings.
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

/// Map the raw `ifconfig` media line to a human-readable speed string.
///
/// `ip` is consulted only for the `auto` branch: an `auto`-negotiated interface
/// with no IP address is treated as disconnected and returns `""`.
fn map_speed_string(ifconfig_output: &str, ip: &str) -> &'static str {
    if ifconfig_output.contains("10G") {
        "10GbE"
    } else if ifconfig_output.contains("5000") {
        "5GbE"
    } else if ifconfig_output.contains("2500") {
        "2.5GbE"
    } else if ifconfig_output.contains("1000") {
        "1GbE"
    } else if ifconfig_output.contains("100") {
        "100Mbps"
    } else if ifconfig_output.contains("10base") {
        "10Mbps"
    } else if !ip.is_empty() && ifconfig_output.contains("auto") {
        // TODO: If location services is enabled for this tool, use
        //  CWWiFiClient.shared()?.interface().transmitRate()
        // to get and display negotiated transmit speed,
        // else just display "auto"
        "auto"
    } else {
        ""
    }
}

/// Entry point: parse CLI flags, collect and sort hardware ports, then display them.
fn main() {
    let config = Config::parse();

    let hardware_ports = HardwarePortList::new()
        .in_service_order()
        .filter_ports(!config.all_ports); // filter to active ports only, unless -all-ports
    print_table(hardware_ports).expect("Failed to output table");
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn make_port(device: &str, ip: &str) -> HardwarePort {
        HardwarePort {
            name: device.to_string(),
            device: device.to_string(),
            ip_address: ip.to_string(),
            mac_address: String::new(),
            speed: String::new(),
            service_order: 0,
        }
    }

    fn port_regex() -> Regex {
        Regex::new(
            r"Hardware Port: ([^\r\n]*)\r?\nDevice: ([^\r\n]*)\r?\nEthernet Address: ([^\r\n]*)\r?\n\r?\n",
        )
        .unwrap()
    }

    #[test]
    fn test_regex_parses_lf() {
        let input =
            "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a1:b2:c3:d4:e5:f6\n\n";
        let caps = port_regex().captures(input).expect("regex should match LF input");
        assert_eq!(&caps[1], "Wi-Fi");
        assert_eq!(&caps[2], "en0");
        assert_eq!(&caps[3], "a1:b2:c3:d4:e5:f6");
    }

    #[test]
    fn test_regex_parses_crlf() {
        let input =
            "Hardware Port: Wi-Fi\r\nDevice: en0\r\nEthernet Address: a1:b2:c3:d4:e5:f6\r\n\r\n";
        let caps = port_regex().captures(input).expect("regex should match CRLF input");
        assert_eq!(&caps[1], "Wi-Fi");
        assert_eq!(&caps[2], "en0");
        assert_eq!(&caps[3], "a1:b2:c3:d4:e5:f6");
    }

    #[test]
    fn test_regex_parses_multiple_ports() {
        let input = concat!(
            "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a1:b2:c3:d4:e5:f6\n\n",
            "Hardware Port: Ethernet\nDevice: en1\nEthernet Address: 11:22:33:44:55:66\n\n",
        );
        let matches: Vec<_> = port_regex().captures_iter(input).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(&matches[0][2], "en0");
        assert_eq!(&matches[1][2], "en1");
    }

    #[test]
    fn test_filter_ports_removes_inactive() {
        let list = HardwarePortList {
            ports: vec![make_port("en0", "192.168.1.1"), make_port("en1", "")],
        };
        let filtered = list.filter_ports(true);
        assert_eq!(filtered.ports.len(), 1);
        assert_eq!(filtered.ports[0].device, "en0");
    }

    #[test]
    fn test_filter_ports_keeps_all() {
        let list = HardwarePortList {
            ports: vec![make_port("en0", "192.168.1.1"), make_port("en1", "")],
        };
        let filtered = list.filter_ports(false);
        assert_eq!(filtered.ports.len(), 2);
    }

    #[test]
    fn test_filter_ports_all_inactive_returns_empty() {
        let list = HardwarePortList {
            ports: vec![make_port("en0", ""), make_port("en1", "")],
        };
        assert!(list.filter_ports(true).ports.is_empty());
    }

    #[test]
    fn test_speed_mapping() {
        assert_eq!(map_speed_string("media: 10GbaseT", ""), "10GbE");
        assert_eq!(map_speed_string("media: 5000baseT", ""), "5GbE");
        assert_eq!(map_speed_string("media: 2500baseT", ""), "2.5GbE");
        assert_eq!(map_speed_string("media: 1000baseT", ""), "1GbE");
        assert_eq!(map_speed_string("media: 100baseTX", ""), "100Mbps");
        assert_eq!(map_speed_string("media: 10baseT", ""), "10Mbps");
        assert_eq!(map_speed_string("media: autoselect", "192.168.1.1"), "auto");
        assert_eq!(map_speed_string("media: autoselect", ""), "");
        assert_eq!(map_speed_string("", ""), "");
    }
}
