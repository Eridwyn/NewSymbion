//! Network discovery and system identification for Symbion agents
//! 
//! This module handles:
//! - Primary MAC address detection with priority (Ethernet > WiFi > Other)
//! - Network interface enumeration with IP addresses  
//! - System identification (hostname, OS, architecture)
//! - Agent ID generation from MAC address

use anyhow::{Result, Context};
use if_addrs::{get_if_addrs, IfAddr};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, debug};

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: String,
    pub ip: String,
    #[serde(rename = "type")]
    pub interface_type: InterfaceType,
}

/// Interface type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceType {
    Ethernet,
    Wireless,
    Loopback,
    Other,
}

/// Complete network discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub primary_mac: String,
    pub interfaces: Vec<NetworkInterface>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub agent_id: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub network: NetworkInfo,
}

/// Priority order for interface selection
const INTERFACE_PRIORITY: &[&str] = &[
    "eth", "en", "ens", "enp", "eno",  // Ethernet (Linux/macOS patterns)
    "wlan", "wlp", "wlo", "wifi",     // WiFi
    "br", "docker", "vir",            // Virtual (lower priority)
];

impl SystemInfo {
    /// Discover complete system information
    pub async fn discover() -> Result<Self> {
        info!("Starting system discovery...");
        
        let network = NetworkInfo::discover()
            .await
            .context("Failed to discover network information")?;
            
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();
            
        let os = std::env::consts::OS.to_string();
        let architecture = std::env::consts::ARCH.to_string();
        
        // Generate agent ID from primary MAC (remove colons)
        let agent_id = network.primary_mac.replace(":", "");
        
        info!("Discovery complete - Agent ID: {}, Hostname: {}, OS: {}", 
              agent_id, hostname, os);
              
        Ok(SystemInfo {
            agent_id,
            hostname,
            os,
            architecture,
            network,
        })
    }
}

impl NetworkInfo {
    /// Discover network interfaces and determine primary MAC
    pub async fn discover() -> Result<Self> {
        debug!("Enumerating network interfaces...");
        
        let if_addrs = get_if_addrs()
            .context("Failed to enumerate network interfaces")?;
            
        let mut interfaces = Vec::new();
        let mut mac_to_interface: HashMap<String, NetworkInterface> = HashMap::new();
        
        // Collect interface information
        for if_addr in if_addrs {
            if if_addr.is_loopback() {
                continue; // Skip loopback for primary selection
            }
            
            let ip = match if_addr.addr {
                IfAddr::V4(v4) => v4.ip.to_string(),
                IfAddr::V6(v6) => v6.ip.to_string(),
            };
            
            // Try to get MAC address for this interface
            if let Some(mac) = Self::get_interface_mac(&if_addr.name).await {
                let mac_str = format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    mac.bytes()[0], mac.bytes()[1], mac.bytes()[2],
                    mac.bytes()[3], mac.bytes()[4], mac.bytes()[5]);
                    
                let interface = NetworkInterface {
                    name: if_addr.name.clone(),
                    mac: mac_str.clone(),
                    ip,
                    interface_type: Self::classify_interface(&if_addr.name),
                };
                
                debug!("Found interface: {} ({})", interface.name, interface.mac);
                interfaces.push(interface.clone());
                mac_to_interface.insert(mac_str, interface);
            }
        }
        
        // Determine primary MAC address based on priority
        let primary_mac = Self::select_primary_mac(&interfaces)?;
        
        info!("Selected primary MAC: {} from {} interfaces", primary_mac, interfaces.len());
        
        Ok(NetworkInfo {
            primary_mac,
            interfaces,
        })
    }
    
    /// Get MAC address for a specific interface name
    async fn get_interface_mac(interface_name: &str) -> Option<MacAddress> {
        // Try interface-specific MAC first
        match mac_address::mac_address_by_name(interface_name) {
            Ok(Some(mac)) => return Some(mac),
            Ok(None) => debug!("No MAC found for interface: {}", interface_name),
            Err(e) => debug!("Error getting MAC for {}: {}", interface_name, e),
        }
        
        None
    }
    
    /// Classify interface type based on name patterns
    fn classify_interface(name: &str) -> InterfaceType {
        let name_lower = name.to_lowercase();
        
        if name_lower.contains("lo") {
            return InterfaceType::Loopback;
        }
        
        // Check for wireless patterns
        if name_lower.contains("wlan") || name_lower.contains("wifi") || 
           name_lower.contains("wlp") || name_lower.contains("wlo") {
            return InterfaceType::Wireless;
        }
        
        // Check for ethernet patterns  
        if name_lower.starts_with("eth") || name_lower.starts_with("en") ||
           name_lower.starts_with("ens") || name_lower.starts_with("enp") ||
           name_lower.starts_with("eno") {
            return InterfaceType::Ethernet;
        }
        
        InterfaceType::Other
    }
    
    /// Select primary MAC address based on interface priority
    fn select_primary_mac(interfaces: &[NetworkInterface]) -> Result<String> {
        if interfaces.is_empty() {
            return Err(anyhow::anyhow!("No network interfaces found"));
        }
        
        // Priority 1: Ethernet interfaces
        for interface in interfaces {
            if matches!(interface.interface_type, InterfaceType::Ethernet) {
                info!("Selected Ethernet interface as primary: {}", interface.name);
                return Ok(interface.mac.clone());
            }
        }
        
        // Priority 2: Wireless interfaces
        for interface in interfaces {
            if matches!(interface.interface_type, InterfaceType::Wireless) {
                info!("Selected WiFi interface as primary: {}", interface.name);
                return Ok(interface.mac.clone());
            }
        }
        
        // Priority 3: Any other interface
        if let Some(interface) = interfaces.first() {
            warn!("No Ethernet/WiFi found, using first interface: {}", interface.name);
            return Ok(interface.mac.clone());
        }
        
        Err(anyhow::anyhow!("No suitable network interface found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interface_classification() {
        assert!(matches!(
            NetworkInfo::classify_interface("eth0"), 
            InterfaceType::Ethernet
        ));
        assert!(matches!(
            NetworkInfo::classify_interface("wlan0"), 
            InterfaceType::Wireless
        ));
        assert!(matches!(
            NetworkInfo::classify_interface("lo"), 
            InterfaceType::Loopback
        ));
    }
    
    #[test]
    fn test_agent_id_generation() {
        let mac = "a1:b2:c3:d4:e5:f6";
        let expected = "a1b2c3d4e5f6";
        assert_eq!(mac.replace(":", ""), expected);
    }
}