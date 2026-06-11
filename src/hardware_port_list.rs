use crate::hardware_port::HardwarePort;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;
use std::str;

/// An ordered collection of hardware network ports discovered on this machine.
pub struct HardwarePortList {
    pub ports: Vec<HardwarePort>,
}

/// Parse the output of `networksetup -listallhardwareports` into
/// `(name, device, mac_address)` tuples, one per hardware port block.
///
/// Each port block is three lines followed by a blank line. The `\r?`
/// handles both LF and CRLF line endings defensively.
fn parse_hardware_ports(stdout: &str) -> Vec<(String, String, String)> {
    let re = Regex::new(
        r"Hardware Port: ([^\r\n]*)\r?\nDevice: ([^\r\n]*)\r?\nEthernet Address: ([^\r\n]*)\r?\n\r?\n",
    )
    .unwrap();
    re.captures_iter(stdout)
        .map(|caps| (caps[1].to_string(), caps[2].to_string(), caps[3].to_string()))
        .collect()
}

/// Parse the output of `networksetup -listnetworkserviceorder | grep Device`
/// into a map from device name to service order index.
///
/// Sample input line: `(Hardware Port: Wi-Fi, Device: en0)`
fn parse_service_order(output: &str) -> HashMap<String, usize> {
    let mut service_order: HashMap<String, usize> = HashMap::new();
    for (i, line) in output.lines().enumerate() {
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

impl HardwarePortList {
    /// Discover all hardware network ports by running `networksetup -listallhardwareports`
    /// and construct a `HardwarePort` for each one.
    ///
    /// The returned list is in the arbitrary order that `networksetup` emits, not
    /// service-preference order; call `in_service_order()` to sort before display.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let output = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;

        let mut ports = Vec::new();
        for (name, device, mac_address) in parse_hardware_ports(&stdout) {
            let mut port = HardwarePort::new(name, device, mac_address);
            port.query_network_state()?;
            ports.push(port);
        }

        Ok(Self { ports })
    }

    /// Re-order ports to match the network service priority set in System Settings.
    ///
    /// Ports absent from the service order (e.g. virtual or inactive interfaces)
    /// are placed at the end by assigning them `usize::MAX` as their sort key.
    pub fn in_service_order(mut self) -> Self {
        fn get_service_order() -> HashMap<String, usize> {
            // uses the shell command: networksetup -listnetworkserviceorder
            //
            // which has sample output containing lines like:
            //      (Hardware Port: Thunderbolt Ethernet Slot 1, Device: en7)
            //      (Hardware Port: Thunderbolt Ethernet Slot 0, Device: en8)
            //      (Hardware Port: Thunderbolt Bridge, Device: bridge0)
            //      (Hardware Port: Wi-Fi, Device: en0)
            let output = Command::new("networksetup")
                .arg("-listnetworkserviceorder")
                .output()
                .unwrap();
            let stdout = str::from_utf8(&output.stdout).unwrap();
            let device_lines: String = stdout
                .lines()
                .filter(|line| line.contains("Device"))
                .collect::<Vec<_>>()
                .join("\n");

            parse_service_order(&device_lines)
        }

        let services_in_order = get_service_order();
        for port in &mut *self.ports {
            if services_in_order.contains_key(&port.device) {
                port.service_order = services_in_order[&port.device];
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
    pub fn filter_ports(self, active_only: bool) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_parse_hardware_ports_lf() {
        let input = "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a1:b2:c3:d4:e5:f6\n\n";
        let ports = parse_hardware_ports(input);
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ("Wi-Fi".to_string(), "en0".to_string(), "a1:b2:c3:d4:e5:f6".to_string()));
    }

    #[test]
    fn test_parse_hardware_ports_crlf() {
        let input =
            "Hardware Port: Wi-Fi\r\nDevice: en0\r\nEthernet Address: a1:b2:c3:d4:e5:f6\r\n\r\n";
        let ports = parse_hardware_ports(input);
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], ("Wi-Fi".to_string(), "en0".to_string(), "a1:b2:c3:d4:e5:f6".to_string()));
    }

    #[test]
    fn test_parse_hardware_ports_multiple() {
        let input = concat!(
            "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a1:b2:c3:d4:e5:f6\n\n",
            "Hardware Port: Ethernet\nDevice: en1\nEthernet Address: 11:22:33:44:55:66\n\n",
        );
        let ports = parse_hardware_ports(input);
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].1, "en0");
        assert_eq!(ports[1].1, "en1");
    }

    #[test]
    fn test_parse_service_order() {
        let input = concat!(
            "(Hardware Port: Thunderbolt Ethernet Slot 1, Device: en7)\n",
            "(Hardware Port: Thunderbolt Ethernet Slot 0, Device: en8)\n",
            "(Hardware Port: Wi-Fi, Device: en0)\n",
        );
        let order = parse_service_order(input);
        assert_eq!(order.get("en7"), Some(&0));
        assert_eq!(order.get("en8"), Some(&1));
        assert_eq!(order.get("en0"), Some(&2));
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
}
