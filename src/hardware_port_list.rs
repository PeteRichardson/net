use crate::hardware_port::HardwarePort;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::{Command, Stdio};
use std::str;

/// An ordered collection of hardware network ports discovered on this machine.
pub struct HardwarePortList {
    pub ports: Vec<HardwarePort>,
}

impl HardwarePortList {
    /// Discover all hardware network ports by running `networksetup -listallhardwareports`
    /// and construct a `HardwarePort` for each one.
    ///
    /// The returned list is in the arbitrary order that `networksetup` emits, not
    /// service-preference order; call `in_service_order()` to sort before display.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut port_data: Vec<HardwarePort> = Vec::new();
        let ports = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()?;
        let stdout = String::from_utf8(ports.stdout)?;

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
            port_data.push(HardwarePort::new(portname, device, mac_address)?)
        }

        //HardwarePortList::sort_by_service_order(&mut port_data);
        Ok(Self { ports: port_data })
    }

    /// Re-order ports to match the network service priority set in System Settings.
    ///
    /// Ports absent from the service order (e.g. virtual or inactive interfaces)
    /// are placed at the end by assigning them `usize::MAX` as their sort key.
    pub fn in_service_order(mut self) -> Self {
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
            let mut networksetup_child = Command::new("networksetup")
                .arg("-listnetworkserviceorder")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let grep_child_one = Command::new("grep")
                .arg("Device")
                .stdin(Stdio::from(networksetup_child.stdout.take().unwrap())) // Pipe through.
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let output = grep_child_one.wait_with_output().unwrap();
            networksetup_child.wait().unwrap();
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

    fn port_regex() -> Regex {
        Regex::new(
            r"Hardware Port: ([^\r\n]*)\r?\nDevice: ([^\r\n]*)\r?\nEthernet Address: ([^\r\n]*)\r?\n\r?\n",
        )
        .unwrap()
    }

    #[test]
    fn test_regex_parses_lf() {
        let input = "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: a1:b2:c3:d4:e5:f6\n\n";
        let caps = port_regex()
            .captures(input)
            .expect("regex should match LF input");
        assert_eq!(&caps[1], "Wi-Fi");
        assert_eq!(&caps[2], "en0");
        assert_eq!(&caps[3], "a1:b2:c3:d4:e5:f6");
    }

    #[test]
    fn test_regex_parses_crlf() {
        let input =
            "Hardware Port: Wi-Fi\r\nDevice: en0\r\nEthernet Address: a1:b2:c3:d4:e5:f6\r\n\r\n";
        let caps = port_regex()
            .captures(input)
            .expect("regex should match CRLF input");
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
}
