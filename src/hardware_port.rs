use std::error::Error;
use std::process::{Command, Stdio};
use std::str;
use tabled::Tabled;

/// Represents a macOS hardware network port and its current network state.
///
/// Fields are populated from `networksetup -listallhardwareports` and
/// supplemental `ipconfig` / `ifconfig` queries. `service_order` is excluded
/// from table output and is used only to sort ports for display.
#[derive(Tabled, Default)]
#[tabled(rename_all = "PascalCase")]
pub struct HardwarePort {
    pub(crate) name: String,
    #[tabled(rename = "IP Address")]
    pub(crate) ip_address: String,
    pub(crate) device: String,
    pub(crate) speed: String,
    #[tabled(rename = "MAC Address")]
    pub(crate) mac_address: String,
    // Not rendered in the table; populated by `in_service_order()` and
    // used as the sort key so ports are listed in network preference order.
    #[tabled(skip)]
    pub(crate) service_order: usize,
}

impl HardwarePort {
    /// Construct a `HardwarePort` from the identifying fields returned by
    /// `networksetup -listallhardwareports`, querying for IP address and
    /// link speed as part of initialization.
    pub(crate) fn new(
        name: String,
        device: String,
        mac_address: String,
    ) -> Result<Self, Box<dyn Error>> {
        let ip_address = HardwarePort::get_ipaddr(&device)?;
        let speed = HardwarePort::get_speed(&device, &ip_address)?;
        Ok(Self {
            name,
            ip_address,
            speed,
            device,
            mac_address,
            service_order: 0,
        })
    }

    /// Return the IPv4 address currently assigned to `device`, or an empty
    /// string if the interface has no address.
    ///
    /// Delegates to `ipconfig getifaddr <device>`.
    fn get_ipaddr(device: &String) -> Result<String, std::io::Error> {
        //ipconfig getifaddr {device}
        let output = Command::new("ipconfig")
            .arg("getifaddr")
            .arg(device)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Return a human-readable link-speed string for `device` (e.g. `"1GbE"`,
    /// `"100Mbps"`), or an empty string if speed cannot be determined.
    ///
    /// Parses the `media` line from `ifconfig <device>`. When the interface
    /// reports `auto` and an IP is present, returns `"auto"` because the
    /// negotiated rate is not accessible without location-services permission.
    fn get_speed(device: &String, ip: &str) -> Result<String, Box<dyn Error>> {
        let output = Command::new("ifconfig")
            .arg(device)
            .stderr(Stdio::null())
            .output()?;
        let stdout = str::from_utf8(&output.stdout)?;
        let media_lines: String = stdout
            .lines()
            .filter(|line| line.contains("media"))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(map_speed_string(&media_lines, ip).to_string())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
