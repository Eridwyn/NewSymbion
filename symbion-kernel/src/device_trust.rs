/**
 * Module Device Trust - Gestion des appareils de confiance
 *
 * Permet de mémoriser les appareils pour ne pas demander le code TOTP
 * pendant 30 jours si l'utilisateur coche "Se souvenir de cet appareil"
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use time::OffsetDateTime;

const DEVICE_TOKENS_FILE: &str = "device_tokens.json";
const DEVICE_TRUST_DURATION_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToken {
    /// Token unique identifiant l'appareil
    pub token: String,
    /// Nom d'utilisateur associé
    pub username: String,
    /// Fingerprint de l'appareil (hash User-Agent)
    pub device_fingerprint: String,
    /// Date de création du token
    pub created_at: i64,
    /// Date d'expiration du token (created_at + 30 jours)
    pub expires_at: i64,
    /// Dernière utilisation du token
    pub last_used_at: i64,
}

impl DeviceToken {
    /// Vérifie si le token est encore valide (non expiré)
    pub fn is_valid(&self) -> bool {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        now < self.expires_at
    }

    /// Vérifie si le token correspond au fingerprint fourni
    pub fn matches_fingerprint(&self, fingerprint: &str) -> bool {
        self.device_fingerprint == fingerprint
    }
}

#[derive(Clone)]
pub struct DeviceTrustManager {
    /// Stockage des tokens : token_id -> DeviceToken
    tokens: Arc<RwLock<HashMap<String, DeviceToken>>>,
}

impl DeviceTrustManager {
    /// Crée un nouveau gestionnaire de device trust
    pub fn new() -> Result<Self> {
        let tokens = Self::load_tokens()?;
        Ok(Self {
            tokens: Arc::new(RwLock::new(tokens)),
        })
    }

    /// Charge les tokens depuis le fichier JSON
    fn load_tokens() -> Result<HashMap<String, DeviceToken>> {
        if !Path::new(DEVICE_TOKENS_FILE).exists() {
            println!("[device-trust] No existing tokens file, starting fresh");
            return Ok(HashMap::new());
        }

        let data = fs::read_to_string(DEVICE_TOKENS_FILE)
            .context("Failed to read device tokens file")?;

        let tokens: HashMap<String, DeviceToken> = serde_json::from_str(&data)
            .context("Failed to parse device tokens file")?;

        println!("[device-trust] Loaded {} device tokens", tokens.len());
        Ok(tokens)
    }

    /// Sauvegarde les tokens dans le fichier JSON
    fn save_tokens(&self) -> Result<()> {
        let tokens = self.tokens.read();
        let json = serde_json::to_string_pretty(&*tokens)
            .context("Failed to serialize device tokens")?;

        fs::write(DEVICE_TOKENS_FILE, json)
            .context("Failed to write device tokens file")?;

        Ok(())
    }

    /// Génère un fingerprint unique pour un appareil
    /// Basé sur le User-Agent du navigateur
    pub fn generate_fingerprint(user_agent: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(user_agent.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Génère un token unique aléatoire
    fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let mut hasher = Sha256::new();
        hasher.update(&random_bytes);
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Crée un nouveau token device pour un utilisateur
    pub fn create_device_token(&self, username: &str, device_fingerprint: &str) -> Result<String> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + (DEVICE_TRUST_DURATION_DAYS * 24 * 3600);

        let token_id = Self::generate_token();

        let device_token = DeviceToken {
            token: token_id.clone(),
            username: username.to_string(),
            device_fingerprint: device_fingerprint.to_string(),
            created_at: now,
            expires_at,
            last_used_at: now,
        };

        let mut tokens = self.tokens.write();
        tokens.insert(token_id.clone(), device_token);
        drop(tokens);

        self.save_tokens()?;

        println!("[device-trust] Created device token for user '{}' (expires in {} days)",
                 username, DEVICE_TRUST_DURATION_DAYS);

        Ok(token_id)
    }

    /// Vérifie si un token device est valide pour un utilisateur et fingerprint
    pub fn verify_device_token(&self, token_id: &str, username: &str, device_fingerprint: &str) -> bool {
        let mut tokens = self.tokens.write();

        let Some(token) = tokens.get_mut(token_id) else {
            return false;
        };

        // Vérifier que le token correspond au bon utilisateur
        if token.username != username {
            println!("[device-trust] Token username mismatch: expected '{}', got '{}'",
                     token.username, username);
            return false;
        }

        // Vérifier que le fingerprint correspond
        if !token.matches_fingerprint(device_fingerprint) {
            println!("[device-trust] Device fingerprint mismatch for user '{}'", username);
            return false;
        }

        // Vérifier que le token n'est pas expiré
        if !token.is_valid() {
            println!("[device-trust] Device token expired for user '{}'", username);
            // Supprimer le token expiré
            tokens.remove(token_id);
            drop(tokens);
            let _ = self.save_tokens();
            return false;
        }

        // Mettre à jour last_used_at
        token.last_used_at = OffsetDateTime::now_utc().unix_timestamp();
        drop(tokens);
        let _ = self.save_tokens();

        println!("[device-trust] Device token valid for user '{}'", username);
        true
    }

    /// Révoque un token device spécifique
    pub fn revoke_device_token(&self, token_id: &str) -> Result<()> {
        let mut tokens = self.tokens.write();

        if tokens.remove(token_id).is_some() {
            drop(tokens);
            self.save_tokens()?;
            println!("[device-trust] Device token revoked: {}", token_id);
        }

        Ok(())
    }

    /// Révoque tous les tokens d'un utilisateur
    pub fn revoke_user_tokens(&self, username: &str) -> Result<usize> {
        let mut tokens = self.tokens.write();

        let before_count = tokens.len();
        tokens.retain(|_, token| token.username != username);
        let removed = before_count - tokens.len();

        if removed > 0 {
            drop(tokens);
            self.save_tokens()?;
            println!("[device-trust] Revoked {} device tokens for user '{}'", removed, username);
        }

        Ok(removed)
    }

    /// Nettoie les tokens expirés
    pub fn cleanup_expired_tokens(&self) -> Result<usize> {
        let mut tokens = self.tokens.write();

        let before_count = tokens.len();
        tokens.retain(|_, token| token.is_valid());
        let removed = before_count - tokens.len();

        if removed > 0 {
            drop(tokens);
            self.save_tokens()?;
            println!("[device-trust] Cleaned up {} expired device tokens", removed);
        }

        Ok(removed)
    }

    /// Compte le nombre de tokens actifs pour un utilisateur
    pub fn count_user_tokens(&self, username: &str) -> usize {
        let tokens = self.tokens.read();
        tokens.values()
            .filter(|t| t.username == username && t.is_valid())
            .count()
    }
}
