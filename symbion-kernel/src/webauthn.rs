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
        let credentials = Self::load_credentials(&storage_path)?;

        Ok(Self {
            webauthn,
            credentials: Arc::new(RwLock::new(credentials)),
            registration_states: Arc::new(RwLock::new(HashMap::new())),
            authentication_states: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
        })
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

    /// Sauvegarde les credentials dans le fichier JSON
    fn save_credentials(&self) -> Result<()> {
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
        // Récupérer TOUTES les passkeys enregistrées (tous utilisateurs)
        let all_passkeys = {
            let creds = self.credentials.read();
            creds
                .values()
                .flat_map(|user_creds| user_creds.iter())
                .map(|stored_cred| stored_cred.credential.clone())
                .collect::<Vec<_>>()
        };

        if all_passkeys.is_empty() {
            anyhow::bail!("No passkeys registered in the system");
        }

        // Générer le challenge d'authentification avec TOUTES les passkeys
        let (rcr, auth_state) = self
            .webauthn
            .start_passkey_authentication(&all_passkeys)?;

        // Stocker l'état temporaire avec clé spéciale "discoverable" + expiration (5 minutes)
        let expires_at = time::OffsetDateTime::now_utc().unix_timestamp() + 300;
        let state_key = format!("discoverable_{}", expires_at);

        let mut states = self.authentication_states.write();
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
