//! MFA (Multi-Factor Authentication) Module
//!
//! Gère l'authentification à deux facteurs basée sur TOTP (Time-based One-Time Password)
//! Compatible avec Google Authenticator, Authy, et autres applications TOTP standard (RFC 6238)

use anyhow::{Context, Result};
use base32::{Alphabet, encode as base32_encode};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use parking_lot::RwLock;
use qrcode::QrCode;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

/// Configuration MFA pour un utilisateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    /// MFA activé ou non
    pub enabled: bool,
    /// Secret TOTP encodé en base32
    pub secret_base32: String,
    /// Codes de récupération (backup codes)
    pub backup_codes: Vec<String>,
    /// Email de récupération (optionnel)
    pub recovery_email: Option<String>,
    /// Timestamp de la configuration initiale
    pub setup_at: i64,
    /// Timestamp de la dernière vérification MFA réussie
    pub last_verified_at: i64,
}

impl MfaConfig {
    /// Crée une nouvelle configuration MFA (non activée par défaut)
    pub fn new_disabled() -> Self {
        Self {
            enabled: false,
            secret_base32: String::new(),
            backup_codes: Vec::new(),
            recovery_email: None,
            setup_at: 0,
            last_verified_at: 0,
        }
    }
}

/// Gestionnaire MFA pour l'application
pub struct MfaManager {
    /// Configurations MFA par utilisateur
    user_configs: Arc<RwLock<HashMap<String, MfaConfig>>>,
    /// Nom de l'application (affiché dans l'authenticator)
    app_name: String,
    /// Issuer (issuer TOTP)
    issuer: String,
}

impl MfaManager {
    /// Crée un nouveau gestionnaire MFA
    pub fn new(app_name: String, issuer: String) -> Self {
        Self {
            user_configs: Arc::new(RwLock::new(HashMap::new())),
            app_name,
            issuer,
        }
    }

    /// Génère un nouveau secret TOTP pour un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur pour lequel générer le secret
    ///
    /// # Returns
    /// Le secret encodé en base32 (à stocker en base de données)
    pub fn generate_secret(&self) -> Result<String> {
        // Générer 20 bytes aléatoires (160 bits)
        let mut rng = rand::thread_rng();
        let secret_bytes: Vec<u8> = (0..20).map(|_| rng.gen::<u8>()).collect();

        // Encoder en base32 (format standard TOTP)
        let secret_base32 = base32_encode(Alphabet::Rfc4648 { padding: false }, &secret_bytes);

        Ok(secret_base32)
    }

    /// Génère un QR code pour la configuration TOTP
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    /// * `secret_base32` - Secret TOTP encodé en base32
    ///
    /// # Returns
    /// String SVG data URI pour affichage dans le frontend
    pub fn generate_qr_code(&self, username: &str, secret_base32: &str) -> Result<String> {
        // Construire l'URL TOTP selon RFC 6238
        // Format: otpauth://totp/{issuer}:{username}?secret={secret}&issuer={issuer}
        let totp_url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}",
            self.issuer, username, secret_base32, self.issuer
        );

        // Générer le QR code
        let qr = QrCode::new(totp_url.as_bytes())
            .context("Failed to generate QR code")?;

        // Convertir en SVG
        let svg = qr.render::<qrcode::render::svg::Color>()
            .min_dimensions(200, 200)
            .build();

        // Encoder en data URI pour affichage direct dans <img src="...">
        let data_uri = format!("data:image/svg+xml;base64,{}", BASE64.encode(svg.as_bytes()));

        Ok(data_uri)
    }

    /// Vérifie un code TOTP
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    /// * `code` - Code à 6 chiffres entré par l'utilisateur
    ///
    /// # Returns
    /// `true` si le code est valide, `false` sinon
    pub fn verify_totp(&self, username: &str, code: &str) -> Result<bool> {
        let configs = self.user_configs.read();

        let config = configs.get(username)
            .context("User MFA config not found")?;

        if !config.enabled {
            return Ok(false);
        }

        // Créer l'instance TOTP avec le secret de l'utilisateur
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // 6 chiffres
            1,  // 1 step (tolérance ±30s)
            30, // 30 secondes par période
            Secret::Encoded(config.secret_base32.clone()).to_bytes()?,
        )?;

        // Vérifier le code avec une fenêtre de tolérance (±1 période = ±30s)
        let is_valid = totp.check_current(code)?;

        if is_valid {
            // Mettre à jour le timestamp de dernière vérification
            drop(configs);
            let mut configs_mut = self.user_configs.write();
            if let Some(config_mut) = configs_mut.get_mut(username) {
                config_mut.last_verified_at = time::OffsetDateTime::now_utc().unix_timestamp();
            }
        }

        Ok(is_valid)
    }

    /// Vérifie un code TOTP directement avec le secret (pas de lookup username)
    ///
    /// Utilisé pour l'authentification où on a déjà le secret depuis users.json
    ///
    /// # Arguments
    /// * `secret_base32` - Secret TOTP en base32
    /// * `code` - Code à 6 chiffres entré par l'utilisateur
    ///
    /// # Returns
    /// `true` si le code est valide, `false` sinon
    pub fn verify_totp_with_secret(&self, secret_base32: &str, code: &str) -> Result<bool> {
        // Créer l'instance TOTP avec le secret fourni
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // 6 chiffres
            1,  // 1 step (tolérance ±30s)
            30, // 30 secondes par période
            Secret::Encoded(secret_base32.to_string()).to_bytes()?,
        )?;

        // Vérifier le code avec une fenêtre de tolérance (±1 période = ±30s)
        let is_valid = totp.check_current(code)?;

        Ok(is_valid)
    }

    /// Génère des codes de récupération (backup codes)
    ///
    /// # Arguments
    /// * `count` - Nombre de codes à générer (recommandé: 10)
    ///
    /// # Returns
    /// Liste de codes de récupération à 8 chiffres
    pub fn generate_backup_codes(&self, count: usize) -> Vec<String> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                // Générer un code à 8 chiffres
                let code: u32 = rng.gen_range(10_000_000..99_999_999);
                code.to_string()
            })
            .collect()
    }

    /// Vérifie un backup code et le consomme (single-use)
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    /// * `code` - Code de récupération à vérifier
    ///
    /// # Returns
    /// `true` si le code est valide et a été consommé, `false` sinon
    pub fn verify_backup_code(&self, username: &str, code: &str) -> Result<bool> {
        let mut configs = self.user_configs.write();

        let config = configs.get_mut(username)
            .context("User MFA config not found")?;

        if !config.enabled {
            return Ok(false);
        }

        // Chercher le code dans la liste
        if let Some(index) = config.backup_codes.iter().position(|c| c == code) {
            // Code trouvé: le supprimer (single-use)
            config.backup_codes.remove(index);
            config.last_verified_at = time::OffsetDateTime::now_utc().unix_timestamp();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Configure MFA pour un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    /// * `config` - Configuration MFA à enregistrer
    pub fn set_user_config(&self, username: String, config: MfaConfig) {
        let mut configs = self.user_configs.write();
        configs.insert(username, config);
    }

    /// Récupère la configuration MFA d'un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    ///
    /// # Returns
    /// Configuration MFA si elle existe, None sinon
    pub fn get_user_config(&self, username: &str) -> Option<MfaConfig> {
        let configs = self.user_configs.read();
        configs.get(username).cloned()
    }

    /// Active MFA pour un utilisateur après vérification du premier code
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    /// * `verification_code` - Code TOTP de vérification initiale
    ///
    /// # Returns
    /// `true` si MFA activé avec succès, `false` si code invalide
    pub fn enable_mfa(&self, username: &str, verification_code: &str) -> Result<bool> {
        // Vérifier le code TOTP avant activation
        if !self.verify_totp(username, verification_code)? {
            return Ok(false);
        }

        // Activer MFA
        let mut configs = self.user_configs.write();
        if let Some(config) = configs.get_mut(username) {
            config.enabled = true;
            config.setup_at = time::OffsetDateTime::now_utc().unix_timestamp();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Désactive MFA pour un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    pub fn disable_mfa(&self, username: &str) {
        let mut configs = self.user_configs.write();
        if let Some(config) = configs.get_mut(username) {
            config.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let secret = manager.generate_secret().expect("Failed to generate secret");

        // Le secret doit être non vide et en base32 valide
        assert!(!secret.is_empty());
        assert!(secret.len() >= 16); // Au moins 16 caractères base32 pour 20 bytes
    }

    #[test]
    fn test_generate_backup_codes() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let codes = manager.generate_backup_codes(10);

        assert_eq!(codes.len(), 10);

        // Vérifier que tous les codes sont à 8 chiffres
        for code in codes {
            assert_eq!(code.len(), 8);
            assert!(code.chars().all(|c| c.is_numeric()));
        }
    }

    #[test]
    fn test_mfa_config_creation() {
        let config = MfaConfig::new_disabled();
        assert!(!config.enabled);
        assert!(config.backup_codes.is_empty());
        assert_eq!(config.setup_at, 0);
    }

    // === K1: Tests MFA étendus ===

    #[test]
    fn test_generate_secret_uniqueness() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let secrets: Vec<String> = (0..20)
            .map(|_| manager.generate_secret().unwrap())
            .collect();
        // All secrets should be unique
        let unique: std::collections::HashSet<_> = secrets.iter().collect();
        assert_eq!(unique.len(), secrets.len(), "Generated secrets should be unique");
    }

    #[test]
    fn test_generate_backup_codes_no_duplicates() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let codes = manager.generate_backup_codes(10);
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len(), "Backup codes should be unique");
    }

    #[test]
    fn test_generate_backup_codes_range() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let codes = manager.generate_backup_codes(100);
        for code in &codes {
            let num: u32 = code.parse().expect("Should be numeric");
            assert!(num >= 10_000_000 && num < 100_000_000,
                "Code {} should be in 8-digit range", num);
        }
    }

    #[test]
    fn test_generate_backup_codes_zero() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let codes = manager.generate_backup_codes(0);
        assert!(codes.is_empty());
    }

    #[test]
    fn test_backup_code_single_use() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let codes = manager.generate_backup_codes(3);
        let code = codes[0].clone();

        // Setup user with MFA enabled
        let config = MfaConfig {
            enabled: true,
            secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
            backup_codes: codes,
            recovery_email: None,
            setup_at: 1000,
            last_verified_at: 0,
        };
        manager.set_user_config("testuser".to_string(), config);

        // First use: should succeed
        assert!(manager.verify_backup_code("testuser", &code).unwrap());
        // Second use: should fail (consumed)
        assert!(!manager.verify_backup_code("testuser", &code).unwrap());
    }

    #[test]
    fn test_backup_code_invalid() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let config = MfaConfig {
            enabled: true,
            secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
            backup_codes: vec!["12345678".to_string()],
            recovery_email: None,
            setup_at: 1000,
            last_verified_at: 0,
        };
        manager.set_user_config("testuser".to_string(), config);

        // Non-existent code
        assert!(!manager.verify_backup_code("testuser", "99999999").unwrap());
        // Empty code
        assert!(!manager.verify_backup_code("testuser", "").unwrap());
    }

    #[test]
    fn test_backup_code_disabled_mfa() {
        let manager = MfaManager::new("Symbion".to_string(), "Symbion".to_string());
        let config = MfaConfig {
            enabled: false, // MFA disabled
            secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
            backup_codes: vec!["12345678".to_string()],
            recovery_email: None,
            setup_at: 1000,
            last_verified_at: 0,
        };
        manager.set_user_config("testuser".to_string(), config);

        // Should return false even with valid code when MFA disabled
        assert!(!manager.verify_backup_code("testuser", "12345678").unwrap());
    }

    #[test]
    fn test_qr_code_format() {
        let manager = MfaManager::new("Symbion".to_string(), "SymbionIoT".to_string());
        let secret = manager.generate_secret().unwrap();
        let qr = manager.generate_qr_code("admin", &secret).unwrap();

        // Should be a data URI with SVG base64
        assert!(qr.starts_with("data:image/svg+xml;base64,"), "QR should be SVG data URI");
        // Decode base64 to verify it's valid
        let b64_part = &qr["data:image/svg+xml;base64,".len()..];
        let decoded = BASE64.decode(b64_part).expect("Should be valid base64");
        let svg = String::from_utf8(decoded).expect("Should be valid UTF-8");
        assert!(svg.contains("<svg"), "Decoded content should be SVG");
    }
}
