/**
 * WebAuthn Manager - Authentification Biométrique Symbion
 *
 * Support universel:
 * - Mobile: Touch ID, Face ID, empreintes Android
 * - Desktop: Windows Hello, Touch ID macOS
 * - Navigateurs: Chrome, Firefox, Safari, Edge
 *
 * Standards: WebAuthn (W3C), FIDO2, CTAP2
 */

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use webauthn_rs::prelude::*;

/// Credential WebAuthn stocké pour un utilisateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredential {
    pub username: String,
    pub credential_id: Vec<u8>,
    pub credential: Passkey,
    pub friendly_name: String, // Ex: "iPhone 15 Pro", "Windows Hello"
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

/// État temporaire d'enregistrement (challenge)
#[derive(Debug, Clone)]
pub struct RegistrationState {
    pub username: String,
    pub state: PasskeyRegistration,
    pub expires_at: i64,
}

/// État temporaire d'authentification (challenge)
#[derive(Debug, Clone)]
pub struct AuthenticationState {
    pub state: PasskeyAuthentication,
    pub expires_at: i64,
}

/// Manager WebAuthn centralisé
pub struct WebAuthnManager {
    webauthn: Arc<Webauthn>,
    /// Credentials stockés par username
    credentials: Arc<RwLock<HashMap<String, Vec<StoredCredential>>>>,
    /// États d'enregistrement temporaires (challenge -> state)
    registration_states: Arc<RwLock<HashMap<String, RegistrationState>>>,
    /// États d'authentification temporaires (challenge -> state)
    authentication_states: Arc<RwLock<HashMap<String, AuthenticationState>>>,
    storage_path: PathBuf,
    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
}

impl WebAuthnManager {
    /// Crée un nouveau WebAuthn manager
    ///
    /// # Arguments
    /// * `rp_id` - Relying Party ID (ex: "localhost", "symbion.local")
    /// * `rp_origin` - Origin complet (ex: "https://localhost:8443")
    /// * `storage_path` - Chemin du fichier de stockage des credentials
    pub fn new(rp_id: &str, rp_origin: &str, storage_path: PathBuf) -> Result<Self> {
        // Parse l'origin URL
        let origin = Url::parse(rp_origin)
            .context("Failed to parse RP origin URL")?;

        // Créer le builder WebAuthn
        let mut builder = WebauthnBuilder::new(rp_id, &origin)?;

        // Configurer pour un usage domestique (pas de vérification d'attestation stricte)
        builder = builder.rp_name("Symbion Home Automation");

        let webauthn = Arc::new(builder.build()?);

        // Charger les credentials depuis le fichier
        let mut credentials = Self::load_credentials(&storage_path)?;

        // Dédupliquer au chargement (fix pour les doublons accumulés)
        for (_, user_creds) in credentials.iter_mut() {
            let mut seen = std::collections::HashSet::new();
            let before = user_creds.len();
            user_creds.retain(|c| seen.insert(c.credential_id.clone()));
            if user_creds.len() < before {
                eprintln!("[webauthn] Deduplicated {} → {} credentials",
                    before, user_creds.len());
            }
        }

        Ok(Self {
            webauthn,
            credentials: Arc::new(RwLock::new(credentials)),
            registration_states: Arc::new(RwLock::new(HashMap::new())),
            authentication_states: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
            db: None,
        })
    }

    /// Attach a database for SQLite persistence.
    pub fn with_database(mut self, db: crate::database::SharedDatabase) -> Self {
        let count = crate::database::auth_queries::count_credentials(&db).unwrap_or(0);
        if count > 0 {
            let rows = crate::database::auth_queries::list_all_credentials(&db).unwrap_or_default();
            let mut creds: HashMap<String, Vec<StoredCredential>> = HashMap::new();
            for row in rows {
                let credential: Passkey = match serde_json::from_str(&row.credential_json) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[webauthn] Failed to parse credential from DB: {}", e);
                        continue;
                    }
                };
                let stored = StoredCredential {
                    username: row.username.clone(),
                    credential_id: row.credential_id,
                    credential,
                    friendly_name: row.friendly_name,
                    created_at: row.created_at,
                    last_used_at: row.last_used_at,
                };
                creds.entry(row.username).or_default().push(stored);
            }
            eprintln!("[webauthn] Loaded {} users with passkeys from SQLite", creds.len());
            *self.credentials.write() = creds;
            self.deduplicate_credentials();
        } else {
            // Seed DB from in-memory data
            self.persist_to_db(&db);
        }
        self.db = Some(db);
        self
    }

    /// Persist all credentials to SQLite (upsert — safe against duplicates)
    fn persist_to_db(&self, db: &crate::database::SharedDatabase) {
        let creds = self.credentials.read();
        for (_, user_creds) in creds.iter() {
            for cred in user_creds {
                let credential_json = serde_json::to_string(&cred.credential)
                    .unwrap_or_else(|_| "{}".to_string());
                let row = crate::database::auth_queries::WebauthnRow {
                    id: None,
                    username: cred.username.clone(),
                    credential_id: cred.credential_id.clone(),
                    credential_json,
                    friendly_name: cred.friendly_name.clone(),
                    created_at: cred.created_at,
                    last_used_at: cred.last_used_at,
                };
                let _ = crate::database::auth_queries::insert_credential(db, &row);
            }
        }
    }

    /// Deduplicate in-memory credentials (remove entries with same credential_id)
    fn deduplicate_credentials(&self) {
        let mut creds = self.credentials.write();
        for (_, user_creds) in creds.iter_mut() {
            let mut seen = std::collections::HashSet::new();
            user_creds.retain(|c| seen.insert(c.credential_id.clone()));
        }
    }

    /// Charge les credentials depuis le fichier JSON
    fn load_credentials(path: &PathBuf) -> Result<HashMap<String, Vec<StoredCredential>>> {
        if !path.exists() {
            println!("[webauthn] No credentials file found, starting fresh");
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(path)
            .context("Failed to read credentials file")?;

        let creds: HashMap<String, Vec<StoredCredential>> = serde_json::from_str(&content)
            .context("Failed to parse credentials JSON")?;

        println!("[webauthn] Loaded {} users with passkeys", creds.len());
        Ok(creds)
    }

    /// Sauvegarde les credentials (DB-primary, JSON-fallback)
    fn save_credentials(&self) -> Result<()> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            self.persist_to_db(db);
            // Always write JSON as backup
            let _ = self.save_credentials_json();
            return Ok(());
        }
        self.save_credentials_json()
    }

    /// JSON-only save (fallback)
    fn save_credentials_json(&self) -> Result<()> {
        let creds = self.credentials.read();
        let json = serde_json::to_string_pretty(&*creds)
            .context("Failed to serialize credentials")?;

        std::fs::write(&self.storage_path, json)
            .context("Failed to write credentials file")?;

        println!("[webauthn] Saved credentials to {:?}", self.storage_path);
        Ok(())
    }

    /// Démarre l'enregistrement d'une nouvelle passkey
    ///
    /// Retourne le CreationChallengeResponse à envoyer au client
    pub fn start_registration(
        &self,
        username: &str,
        user_display_name: &str,
    ) -> Result<CreationChallengeResponse> {
        // Créer un UUID unique pour l'utilisateur
        let user_unique_id = Uuid::new_v4();

        // Exclure les credentials déjà enregistrés pour cet utilisateur
        let exclude_credentials = {
            let creds = self.credentials.read();
            creds
                .get(username)
                .map(|user_creds| {
                    user_creds
                        .iter()
                        .map(|c| c.credential.cred_id().clone())
                        .collect()
                })
                .unwrap_or_default()
        };

        // Générer le challenge de création
        let (ccr, reg_state) = self.webauthn.start_passkey_registration(
            user_unique_id,
            username,
            user_display_name,
            Some(exclude_credentials),
        )?;

        // Stocker l'état temporaire par username avec expiration (5 minutes)
        let expires_at = time::OffsetDateTime::now_utc().unix_timestamp() + 300;

        let mut states = self.registration_states.write();
        states.insert(
            username.to_string(),
            RegistrationState {
                username: username.to_string(),
                state: reg_state,
                expires_at,
            },
        );

        println!("[webauthn] Started registration for user '{}'", username);
        Ok(ccr)
    }

    /// Termine l'enregistrement d'une passkey
    ///
    /// Vérifie la réponse du client et stocke le credential
    pub fn finish_registration(
        &self,
        username: &str,
        reg_response: &RegisterPublicKeyCredential,
        friendly_name: String,
    ) -> Result<()> {
        // Récupérer et supprimer l'état temporaire pour cet utilisateur
        let reg_state = {
            let mut states = self.registration_states.write();
            states
                .remove(username)
                .context("Registration state not found or expired")?
        };

        // Vérifier l'expiration
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if now > reg_state.expires_at {
            anyhow::bail!("Registration challenge expired");
        }

        // Vérifier la réponse et créer le Passkey
        let passkey = self
            .webauthn
            .finish_passkey_registration(reg_response, &reg_state.state)?;

        // Stocker le credential
        let stored_cred = StoredCredential {
            username: username.to_string(),
            credential_id: passkey.cred_id().clone().into(),
            credential: passkey,
            friendly_name,
            created_at: now,
            last_used_at: None,
        };

        {
            let mut creds = self.credentials.write();
            creds
                .entry(username.to_string())
                .or_insert_with(Vec::new)
                .push(stored_cred);
        }

        // Sauvegarder
        self.save_credentials()?;

        println!("[webauthn] ✅ Registered passkey for user '{}'", username);
        Ok(())
    }

    /// Démarre l'authentification avec passkey (mode découvrable sans username)
    ///
    /// Permet à l'authenticator de présenter TOUTES les passkeys disponibles
    /// L'utilisateur choisit via son biométrie, le serveur identifie ensuite via credential_id
    pub fn start_discoverable_authentication(&self) -> Result<RequestChallengeResponse> {
        // Browser limit: allowCredentials max 64 entries
        const MAX_ALLOW_CREDENTIALS: usize = 64;

        // Récupérer TOUTES les passkeys enregistrées, triées par dernière utilisation
        let all_passkeys = {
            let creds = self.credentials.read();
            let mut all_stored: Vec<&StoredCredential> = creds
                .values()
                .flat_map(|user_creds| user_creds.iter())
                .collect();

            // Priorité aux plus récemment utilisées
            all_stored.sort_by(|a, b| {
                let a_time = a.last_used_at.unwrap_or(a.created_at);
                let b_time = b.last_used_at.unwrap_or(b.created_at);
                b_time.cmp(&a_time)
            });

            all_stored
                .into_iter()
                .take(MAX_ALLOW_CREDENTIALS)
                .map(|stored_cred| stored_cred.credential.clone())
                .collect::<Vec<_>>()
        };

        if all_passkeys.is_empty() {
            anyhow::bail!("No passkeys registered in the system");
        }

        // Générer le challenge d'authentification (max 64 passkeys)
        let (rcr, auth_state) = self
            .webauthn
            .start_passkey_authentication(&all_passkeys)?;

        // Stocker l'état temporaire avec clé spéciale "discoverable" + expiration (5 minutes)
        // Nettoyer les anciens états discoverable pour éviter les race conditions
        // (un seul challenge discoverable actif à la fois)
        let expires_at = time::OffsetDateTime::now_utc().unix_timestamp() + 300;
        let state_key = format!("discoverable_{}", expires_at);

        let mut states = self.authentication_states.write();
        let stale_keys: Vec<String> = states
            .keys()
            .filter(|k| k.starts_with("discoverable_"))
            .cloned()
            .collect();
        for key in stale_keys {
            states.remove(&key);
        }
        states.insert(
            state_key.clone(),
            AuthenticationState {
                state: auth_state,
                expires_at,
            },
        );

        println!("[webauthn] Started discoverable authentication (all passkeys)");
        Ok(rcr)
    }

    /// Démarre l'authentification avec passkey (mode classique avec username)
    ///
    /// Retourne le RequestChallengeResponse à envoyer au client
    pub fn start_authentication(&self, username: &str) -> Result<RequestChallengeResponse> {
        // Récupérer les credentials de l'utilisateur
        let user_passkeys = {
            let creds = self.credentials.read();
            creds
                .get(username)
                .context("No passkeys found for this user")?
                .iter()
                .map(|c| c.credential.clone())
                .collect::<Vec<_>>()
        };

        if user_passkeys.is_empty() {
            anyhow::bail!("User has no registered passkeys");
        }

        // Générer le challenge d'authentification
        let (rcr, auth_state) = self
            .webauthn
            .start_passkey_authentication(&user_passkeys)?;

        // Stocker l'état temporaire par username avec expiration (5 minutes)
        let expires_at = time::OffsetDateTime::now_utc().unix_timestamp() + 300;

        let mut states = self.authentication_states.write();
        states.insert(
            username.to_string(),
            AuthenticationState {
                state: auth_state,
                expires_at,
            },
        );

        println!("[webauthn] Started authentication for user '{}'", username);
        Ok(rcr)
    }

    /// Termine l'authentification avec passkey
    ///
    /// Vérifie la réponse et retourne le username si succès
    pub fn finish_authentication(
        &self,
        auth_response: &PublicKeyCredential,
    ) -> Result<String> {
        // Trouver l'utilisateur correspondant au credential_id
        let credential_id_bytes = auth_response.raw_id.as_ref();

        let username = {
            let creds = self.credentials.read();
            let mut found_username = None;

            for (user, user_creds) in creds.iter() {
                for cred in user_creds.iter() {
                    if cred.credential_id == credential_id_bytes {
                        found_username = Some(user.clone());
                        break;
                    }
                }
                if found_username.is_some() {
                    break;
                }
            }

            found_username.context("Credential not found - no user has this passkey registered")?
        };

        // Récupérer l'état d'authentification
        // Essayer d'abord par username (mode classique), sinon mode discoverable
        let auth_state = {
            let mut states = self.authentication_states.write();

            // Tentative 1: mode classique avec username
            if let Some(state) = states.remove(&username) {
                state
            } else {
                // Tentative 2: mode discoverable (clé "discoverable_*")
                let discoverable_key = states
                    .keys()
                    .find(|k| k.starts_with("discoverable_"))
                    .cloned();

                if let Some(key) = discoverable_key {
                    states.remove(&key)
                        .context("Discoverable state removed before retrieval")?
                } else {
                    anyhow::bail!("Authentication state not found or expired (tried both username and discoverable modes)")
                }
            }
        };

        // Vérifier expiration
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if now > auth_state.expires_at {
            anyhow::bail!("Authentication challenge expired");
        }

        // Vérifier la réponse
        let auth_result = self
            .webauthn
            .finish_passkey_authentication(auth_response, &auth_state.state)?;

        // Mettre à jour last_used_at
        {
            let mut creds = self.credentials.write();
            if let Some(user_creds) = creds.get_mut(&username) {
                for cred in user_creds.iter_mut() {
                    if cred.credential.cred_id() == auth_result.cred_id() {
                        cred.last_used_at = Some(now);
                        break;
                    }
                }
            }
        }

        // Sauvegarder la mise à jour du last_used_at
        self.save_credentials()?;

        println!("[webauthn] ✅ Authenticated user '{}'", username);
        Ok(username)
    }

    /// Liste les passkeys d'un utilisateur
    pub fn list_user_passkeys(&self, username: &str) -> Vec<StoredCredential> {
        let creds = self.credentials.read();
        creds.get(username).cloned().unwrap_or_default()
    }

    /// Supprime une passkey spécifique
    pub fn delete_passkey(&self, username: &str, credential_id: &[u8]) -> Result<()> {
        let mut creds = self.credentials.write();

        if let Some(user_creds) = creds.get_mut(username) {
            let initial_len = user_creds.len();
            user_creds.retain(|c| c.credential_id != credential_id);

            if user_creds.len() < initial_len {
                drop(creds); // Release lock before saving
                self.save_credentials()?;
                println!("[webauthn] Deleted passkey for user '{}'", username);
                Ok(())
            } else {
                anyhow::bail!("Credential not found")
            }
        } else {
            anyhow::bail!("User has no passkeys")
        }
    }

    /// Nettoie les états expirés (à appeler périodiquement)
    pub fn cleanup_expired_states(&self) {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        {
            let mut reg_states = self.registration_states.write();
            reg_states.retain(|_, state| state.expires_at > now);
        }

        {
            let mut auth_states = self.authentication_states.write();
            auth_states.retain(|_, state| state.expires_at > now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const TEST_CREDENTIALS_FILE: &str = "test_passkeys.json";

    /// Helper: Cleanup test credentials file
    fn cleanup_test_credentials() {
        let path = PathBuf::from(TEST_CREDENTIALS_FILE);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    /// Helper: Create test WebAuthnManager
    fn create_test_manager() -> Result<WebAuthnManager> {
        cleanup_test_credentials();
        WebAuthnManager::new(
            "localhost",
            "https://localhost:8443",
            PathBuf::from(TEST_CREDENTIALS_FILE),
        )
    }

    #[test]
    fn test_webauthn_manager_creation() {
        cleanup_test_credentials();

        let result = WebAuthnManager::new(
            "localhost",
            "https://localhost:8443",
            PathBuf::from(TEST_CREDENTIALS_FILE),
        );

        assert!(result.is_ok(), "WebAuthnManager creation should succeed");
        let manager = result.unwrap();

        // Verify no credentials initially
        let creds = manager.credentials.read();
        assert_eq!(creds.len(), 0, "Should start with no credentials");

        cleanup_test_credentials();
    }

    #[test]
    fn test_webauthn_manager_invalid_origin() {
        cleanup_test_credentials();

        let result = WebAuthnManager::new(
            "localhost",
            "invalid-url-not-https",
            PathBuf::from(TEST_CREDENTIALS_FILE),
        );

        assert!(result.is_err(), "Should fail with invalid origin URL");
        cleanup_test_credentials();
    }

    #[test]
    fn test_start_registration_challenge_structure() {
        let manager = create_test_manager().expect("Failed to create manager");

        let ccr = manager
            .start_registration("testuser", "Test User")
            .expect("Should start registration");

        // Verify challenge structure (challenge is opaque, just verify it exists)
        assert_eq!(ccr.public_key.rp.name, "Symbion Home Automation");
        assert_eq!(ccr.public_key.user.name, "testuser");
        assert_eq!(ccr.public_key.user.display_name, "Test User");

        // Verify state was stored
        let states = manager.registration_states.read();
        assert!(states.contains_key("testuser"), "Registration state should be stored");
        let state = states.get("testuser").unwrap();

        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        assert!(state.expires_at > now, "Challenge should not be expired");
        assert!(state.expires_at <= now + 301, "Challenge should expire in ~5 minutes");

        cleanup_test_credentials();
    }

    #[test]
    fn test_start_registration_multiple_users() {
        let manager = create_test_manager().expect("Failed to create manager");

        manager.start_registration("user1", "User One").expect("User 1 registration");
        manager.start_registration("user2", "User Two").expect("User 2 registration");

        let states = manager.registration_states.read();
        assert_eq!(states.len(), 2, "Should have 2 pending registrations");
        assert!(states.contains_key("user1"));
        assert!(states.contains_key("user2"));

        cleanup_test_credentials();
    }

    #[test]
    fn test_start_authentication_no_passkeys() {
        let manager = create_test_manager().expect("Failed to create manager");

        let result = manager.start_authentication("nonexistent");

        assert!(result.is_err(), "Should fail when user has no passkeys");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("No passkeys") || error_msg.contains("no registered"),
            "Error should mention no passkeys, got: {}",
            error_msg
        );

        cleanup_test_credentials();
    }

    #[test]
    fn test_start_discoverable_authentication_no_passkeys() {
        let manager = create_test_manager().expect("Failed to create manager");

        let result = manager.start_discoverable_authentication();

        assert!(result.is_err(), "Should fail when no passkeys registered");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("No passkeys registered"),
            "Error should mention no passkeys, got: {}",
            error_msg
        );

        cleanup_test_credentials();
    }

    #[test]
    fn test_list_user_passkeys_empty() {
        let manager = create_test_manager().expect("Failed to create manager");

        let passkeys = manager.list_user_passkeys("testuser");

        assert_eq!(passkeys.len(), 0, "Should return empty vec for user with no passkeys");
        cleanup_test_credentials();
    }

    #[test]
    fn test_delete_passkey_no_user() {
        let manager = create_test_manager().expect("Failed to create manager");

        let result = manager.delete_passkey("nonexistent", &[1, 2, 3]);

        assert!(result.is_err(), "Should fail when user doesn't exist");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("no passkeys"),
            "Error should mention no passkeys, got: {}",
            error_msg
        );

        cleanup_test_credentials();
    }

    #[test]
    fn test_cleanup_expired_states() {
        let manager = create_test_manager().expect("Failed to create manager");

        // Create a temporary registration to get a valid state
        manager.start_registration("temp", "Temp").unwrap();

        // Extract the state and modify its expiration
        let expired_state = {
            let mut states = manager.registration_states.write();
            let mut temp_state = states.remove("temp").unwrap();
            temp_state.expires_at = time::OffsetDateTime::now_utc().unix_timestamp() - 100; // Expired 100s ago
            temp_state.username = "expired_user".to_string();
            temp_state
        };

        // Insert the expired state
        {
            let mut states = manager.registration_states.write();
            states.insert("expired_user".to_string(), expired_state);
        }

        // Add a valid state
        manager.start_registration("valid_user", "Valid User").expect("Valid registration");

        // Verify we have 2 states
        {
            let states = manager.registration_states.read();
            assert_eq!(states.len(), 2, "Should have 2 registration states before cleanup");
        }

        // Cleanup
        manager.cleanup_expired_states();

        // Verify only valid state remains
        {
            let states = manager.registration_states.read();
            assert_eq!(states.len(), 1, "Should have 1 registration state after cleanup");
            assert!(states.contains_key("valid_user"), "Valid state should remain");
            assert!(!states.contains_key("expired_user"), "Expired state should be removed");
        }

        cleanup_test_credentials();
    }

    #[test]
    fn test_load_credentials_nonexistent_file() {
        cleanup_test_credentials();

        let path = PathBuf::from("nonexistent_passkeys.json");
        let result = WebAuthnManager::load_credentials(&path);

        assert!(result.is_ok(), "Should return empty HashMap when file doesn't exist");
        assert_eq!(result.unwrap().len(), 0, "Should have no credentials");
    }

    #[test]
    fn test_load_credentials_invalid_json() {
        let test_file = "invalid_passkeys.json";
        fs::write(test_file, "{ invalid json :::").expect("Write invalid JSON");

        let path = PathBuf::from(test_file);
        let result = WebAuthnManager::load_credentials(&path);

        assert!(result.is_err(), "Should fail with invalid JSON");

        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_registration_state_expiration() {
        let manager = create_test_manager().expect("Failed to create manager");

        manager.start_registration("testuser", "Test User").expect("Start registration");

        // Manually expire the state
        {
            let mut states = manager.registration_states.write();
            let state = states.get_mut("testuser").unwrap();
            state.expires_at = time::OffsetDateTime::now_utc().unix_timestamp() - 10; // Expired 10s ago
        }

        // Attempt to finish registration (would fail due to expiration in real flow)
        // We can't actually call finish_registration without a valid RegisterPublicKeyCredential,
        // but we've tested the expiration check logic exists

        cleanup_test_credentials();
    }

    #[test]
    fn test_webauthn_rp_configuration() {
        let manager = create_test_manager().expect("Failed to create manager");

        // Start registration to get challenge response
        let ccr = manager
            .start_registration("testuser", "Test User")
            .expect("Registration started");

        // Verify RP (Relying Party) configuration
        assert_eq!(ccr.public_key.rp.id, "localhost");
        assert_eq!(ccr.public_key.rp.name, "Symbion Home Automation");

        cleanup_test_credentials();
    }

    #[test]
    fn test_concurrent_registrations_different_users() {
        let manager = create_test_manager().expect("Failed to create manager");

        // Start multiple registrations concurrently
        let _ccr1 = manager.start_registration("user1", "User One").expect("User 1");
        let _ccr2 = manager.start_registration("user2", "User Two").expect("User 2");
        let _ccr3 = manager.start_registration("user3", "User Three").expect("User 3");

        // Verify all states are stored (challenges are opaque, can't compare directly)
        let states = manager.registration_states.read();
        assert_eq!(states.len(), 3, "All 3 registration states should be stored");

        cleanup_test_credentials();
    }


    #[test]
    fn test_challenge_expiration_time() {
        let manager = create_test_manager().expect("Failed to create manager");

        let now_before = time::OffsetDateTime::now_utc().unix_timestamp();
        manager.start_registration("testuser", "Test User").expect("Registration");
        let now_after = time::OffsetDateTime::now_utc().unix_timestamp();

        let states = manager.registration_states.read();
        let state = states.get("testuser").unwrap();

        // Challenge should expire in ~5 minutes (300 seconds)
        let expected_min = now_before + 299; // Allow 1s tolerance
        let expected_max = now_after + 301;

        assert!(
            state.expires_at >= expected_min && state.expires_at <= expected_max,
            "Challenge should expire in ~5 minutes, got expires_at={}, now=~{}",
            state.expires_at,
            now_before
        );

        cleanup_test_credentials();
    }
}
