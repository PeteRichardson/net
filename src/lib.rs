//! Core data types for discovering and describing macOS network hardware ports.
//!
//! [`HardwarePortList::new`] gathers ports from the system, [`HardwarePortList::in_service_order`]
//! and [`HardwarePortList::filter_ports`] shape the list for display, and each
//! [`HardwarePort`] holds one port's identity and live network state.

pub mod error;
pub mod hardware_port;
pub mod hardware_port_list;

pub use error::NetError;
pub use hardware_port::HardwarePort;
pub use hardware_port_list::HardwarePortList;
