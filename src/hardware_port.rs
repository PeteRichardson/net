//! A single network hardware port and the system queries used to populate it.

use crate::command::run_command_allow_failure;
use crate::error::NetError;
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
    /// `networksetup -listallhardwareports`.
    ///
    /// `ip_address` and `speed` are left empty; call `query_network_state()`
    /// to populate them from the live system.
    pub(crate) fn new(name: String, device: String, mac_address: String) -> Self {
        Self {
            name,
            device,
            mac_address,
            ip_address: String::new(),
            speed: String::new(),
            service_order: 0,
        }
    }

    /// Query the live system for this port's current IP address and link
    /// speed, and store them on `self`.
    pub(crate) fn query_network_state(&mut self) -> Result<(), NetError> {
        self.ip_address = HardwarePort::get_ipaddr(&self.device)?;
        self.speed = HardwarePort::get_speed(&self.device, &self.ip_address)?;
        Ok(())
    }

    /// Return the IPv4 address currently assigned to `device`, or an empty
    /// string if the interface has no address.
    ///
    /// Delegates to `ipconfig getifaddr <device>`, which exits nonzero when
    /// the interface has no address — a normal state, so that case is
    /// deliberately folded into the empty-string result rather than an error.
    fn get_ipaddr(device: &str) -> Result<String, NetError> {
        let output = run_command_allow_failure("ipconfig", &["getifaddr", device])?;
        Ok(output.trim().to_string())
    }

    /// Return a human-readable link-speed string for `device` (e.g. `"1GbE"`,
    /// `"100Mbps"`), or an empty string if speed cannot be determined.
    ///
    /// Parses the `media` line from `ifconfig <device>`. When the interface
    /// reports `auto` and an IP is present, returns `"auto"` because the
    /// negotiated rate is not accessible without location-services permission.
    fn get_speed(device: &str, ip: &str) -> Result<String, NetError> {
        // `ifconfig` exits nonzero for devices it doesn't recognize (some
        // entries from networksetup); treat that as "no speed", not an error.
        let stdout = run_command_allow_failure("ifconfig", &[device])?;
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
    // Order matters: e.g. "1000" is a substring-match away from "100", so the
    // larger/more-specific speeds must be checked first.
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
    fn test_new_sets_identity_fields_and_defaults() {
        let port = HardwarePort::new(
            "Wi-Fi".to_string(),
            "en0".to_string(),
            "a1:b2:c3:d4:e5:f6".to_string(),
        );
        assert_eq!(port.name, "Wi-Fi");
        assert_eq!(port.device, "en0");
        assert_eq!(port.mac_address, "a1:b2:c3:d4:e5:f6");
        assert_eq!(port.ip_address, "");
        assert_eq!(port.speed, "");
        assert_eq!(port.service_order, 0);
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
