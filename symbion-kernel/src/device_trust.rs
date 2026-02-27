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
    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
}

impl DeviceTrustManager {
    /// Crée un nouveau gestionnaire de device trust
    pub fn new() -> Result<Self> {
        let tokens = Self::load_tokens()?;
        Ok(Self {
            tokens: Arc::new(RwLock::new(tokens)),
            db: None,
        })
    }

    /// Attach a database for SQLite persistence.
    pub fn with_database(mut self, db: crate::database::SharedDatabase) -> Self {
        // Try to load all tokens from DB
        let conn = db.conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM device_tokens", [], |row| row.get(0))
            .unwrap_or(0);
        drop(conn);

        if count > 0 {
            // Load all tokens from DB
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT token, username, device_fingerprint, created_at, expires_at, last_used_at FROM device_tokens"
            ).ok();
            if let Some(ref mut stmt) = stmt {
                let mut tokens = HashMap::new();
                let rows = stmt.query_map::<DeviceToken, _, _>([], |row| {
                    Ok(DeviceToken {
                        token: row.get(0)?,
                        username: row.get(1)?,
                        device_fingerprint: row.get(2)?,
                        created_at: row.get(3)?,
                        expires_at: row.get(4)?,
                        last_used_at: row.get(5)?,
                    })
                });
                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        tokens.insert(row.token.clone(), row);
                    }
                    eprintln!("[device-trust] Loaded {} tokens from SQLite", tokens.len());
                    *self.tokens.write() = tokens;
                }
            }
        } else {
            // DB empty — seed from in-memory data (loaded from JSON)
            let tokens = self.tokens.read();
            for token in tokens.values() {
                let row = crate::database::auth_queries::DeviceTokenRow {
                    token: token.token.clone(),
                    username: token.username.clone(),
                    device_fingerprint: token.device_fingerprint.clone(),
                    created_at: token.created_at,
                    expires_at: token.expires_at,
                    last_used_at: token.last_used_at,
                };
                let _ = crate::database::auth_queries::upsert_device_token(&db, &row);
            }
            if !tokens.is_empty() {
                eprintln!("[device-trust] Seeded {} tokens to SQLite", tokens.len());
            }
        }
        self.db = Some(db);
        self
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

    /// Save tokens — DB primary, JSON fallback.
    fn save_tokens(&self) -> Result<()> {
        let tokens = self.tokens.read();

        // Try SQLite first
        if let Some(ref db) = self.db {
            let mut db_ok = true;
            for (_, token) in tokens.iter() {
                let row = crate::database::auth_queries::DeviceTokenRow {
                    token: token.token.clone(),
                    username: token.username.clone(),
                    device_fingerprint: token.device_fingerprint.clone(),
                    created_at: token.created_at,
                    expires_at: token.expires_at,
                    last_used_at: token.last_used_at,
                };
                if let Err(e) = crate::database::auth_queries::upsert_device_token(db, &row) {
                    eprintln!("[device-trust] SQLite save failed, falling back to JSON: {}", e);
                    db_ok = false;
                    break;
                }
            }
            if db_ok {
                let _ = self.save_tokens_json(&tokens);
                return Ok(());
            }
        }

        self.save_tokens_json(&tokens)
    }

    fn save_tokens_json(&self, tokens: &HashMap<String, DeviceToken>) -> Result<()> {
        let json = serde_json::to_string_pretty(tokens)
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
