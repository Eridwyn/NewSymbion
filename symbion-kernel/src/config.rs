/**
 * CONFIGURATION KERNEL - Chargement et gestion des paramètres Symbion
 * 
 * RÔLE :
 * Ce module gère la configuration centralisée du kernel Symbion depuis un fichier YAML.
 * Il définit les paramètres pour les hosts, MQTT et Wake-on-LAN avec fallback par défaut.
 * 
 * FONCTIONNEMENT :
 * - Lecture de kernel.yaml (ou variable SYMBION_KERNEL_CONFIG)
 * - Parsing YAML -> structures typées avec serde
 * - Fallback vers configuration par défaut si fichier absent/invalide
 * - Configuration partagée entre tous les modules du kernel
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Centralise TOUTE la config : MQTT broker, hosts monitoring, WOL
 * 🎯 Hot-reload friendly : rechargement sans redémarrer le kernel
 * 🎯 Environnement flexible : dev/prod via variable d'environnement
 * 🎯 Robustesse : pas de crash si config absente ou malformée
 * 
 * EXEMPLE KERNEL.YAML :
 * ```yaml
 * mqtt:
 *   host: "192.168.1.100"
 *   port: 1883
 * hosts:
 *   desktop-w11:
 *     mac: "AA:BB:CC:DD:EE:FF"
 *     hint: "192.168.1.44"
 * wol:
 *   command: "wakeonlan {mac}"
 * ```
 */

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::fs;

/// Configuration principale du kernel Symbion
/// Contient toutes les sections : hosts, MQTT, Wake-on-LAN
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostsConfig {
    /// Map des hosts à monitorer : host_id -> config host
    pub hosts: HashMap<String, HostConf>,
    /// Configuration Wake-on-LAN (commande à exécuter)
    pub wol: Option<WolConf>,
    /// Configuration du broker MQTT (host, port)
    pub mqtt: Option<MqttConf>,
}

/// Configuration d'un host spécifique à monitorer
/// Contient les infos nécessaires au WOL et au monitoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConf {
    /// Adresse MAC pour Wake-on-LAN (format AA:BB:CC:DD:EE:FF)
    pub mac: String,
    /// IP hint optionnel pour optimiser le réveil réseau
    pub hint: Option<String>,
}

/// Configuration Wake-on-LAN
/// Définit la commande système à exécuter pour réveiller les machines
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WolConf {
    /// Commande shell avec placeholders : {host_id}, {mac}, {hint}
    /// Exemple: "wakeonlan {mac}" ou "/usr/bin/etherwake {mac}"
    pub command: String,
}

/// Configuration du broker MQTT
/// Définit où se connecter pour les événements Symbion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MqttConf {
    /// Hostname ou IP du broker MQTT
    pub host: String,
    /// Port du broker (généralement 1883 non-TLS, 8883 TLS)
    pub port: u16,
}

impl Default for HostsConfig {
    /// Configuration par défaut si aucun fichier kernel.yaml trouvé
    /// MQTT localhost:1883, pas de hosts ni WOL configurés
    fn default() -> Self {
        Self {
            hosts: HashMap::new(),
            wol: None,
            mqtt: Some(MqttConf { 
                host: "localhost".into(), 
                port: 1883 
            }),
        }
    }
}

/// Charge la configuration depuis le fichier YAML
/// Gère les erreurs gracieusement avec fallback vers config par défaut
pub async fn load_config() -> HostsConfig {
    // Chemin configurable via variable d'environnement
    let path = std::env::var("SYMBION_KERNEL_CONFIG")
        .unwrap_or_else(|_| "kernel.yaml".into());
    
    if Path::new(&path).exists() {
        // Lecture du fichier YAML
        let txt = fs::read_to_string(&path).await.unwrap_or_default();
        if txt.trim().is_empty() { 
            return HostsConfig::default(); 
        }
        
        // Parsing YAML -> structures Rust avec fallback
        serde_yaml::from_str(&txt).unwrap_or_else(|e| {
            eprintln!("[config] YAML invalide dans {}: {}", path, e);
            eprintln!("[config] utilisation de la config par défaut");
            HostsConfig::default()
        })
    } else {
        eprintln!("[config] fichier {} non trouvé, config par défaut", path);
        HostsConfig::default()
    }
}
