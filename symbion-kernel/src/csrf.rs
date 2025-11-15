//! CSRF (Cross-Site Request Forgery) Protection Module
//!
//! Implémente une protection CSRF basée sur des nonces (tokens à usage unique)
//! Les nonces ont une durée de vie limitée (5 minutes) et sont liés à l'utilisateur

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Durée de vie d'un nonce CSRF en secondes (5 minutes)
const CSRF_NONCE_TTL: i64 = 300;

/// Nonce CSRF avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfNonce {
    /// Token UUID unique
    pub token: String,
    /// Nom d'utilisateur associé
    pub username: String,
    /// Timestamp de création (unix timestamp)
    pub created_at: i64,
    /// Timestamp d'expiration (unix timestamp)
    pub expires_at: i64,
    /// Déjà utilisé (single-use)
    pub used: bool,
}

impl CsrfNonce {
    /// Crée un nouveau nonce CSRF
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur pour lequel créer le nonce
    pub fn new(username: String) -> Self {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let token = Uuid::new_v4().to_string();

        Self {
            token,
            username,
            created_at: now,
            expires_at: now + CSRF_NONCE_TTL,
            used: false,
        }
    }

    /// Vérifie si le nonce est expiré
    pub fn is_expired(&self) -> bool {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        now > self.expires_at
    }

    /// Vérifie si le nonce est valide (non expiré et non utilisé)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.used
    }
}

/// Gestionnaire CSRF pour l'application
pub struct CsrfManager {
    /// Nonces actifs indexés par token
    nonces: Arc<RwLock<HashMap<String, CsrfNonce>>>,
}

impl CsrfManager {
    /// Crée un nouveau gestionnaire CSRF
    pub fn new() -> Self {
        Self {
            nonces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Génère un nouveau nonce CSRF pour un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur pour lequel générer le nonce
    ///
    /// # Returns
    /// Token CSRF à envoyer au client
    pub fn generate_nonce(&self, username: String) -> String {
        let nonce = CsrfNonce::new(username);
        let token = nonce.token.clone();

        let mut nonces = self.nonces.write();
        nonces.insert(token.clone(), nonce);

        token
    }

    /// Vérifie et consomme un nonce CSRF
    ///
    /// # Arguments
    /// * `token` - Token CSRF à vérifier
    /// * `username` - Nom d'utilisateur attendu
    ///
    /// # Returns
    /// `Ok(true)` si le nonce est valide et a été consommé
    /// `Ok(false)` si le nonce est invalide ou expiré
    /// `Err(_)` en cas d'erreur
    pub fn verify_and_consume(&self, token: &str, username: &str) -> Result<bool> {
        let mut nonces = self.nonces.write();

        // Récupérer le nonce
        let nonce = match nonces.get_mut(token) {
            Some(n) => n,
            None => return Ok(false), // Token inconnu
        };

        // Vérifier que le nonce est valide
        if !nonce.is_valid() {
            nonces.remove(token); // Nettoyer le nonce expiré/utilisé
            return Ok(false);
        }

        // Vérifier que le nonce appartient bien à l'utilisateur
        if nonce.username != username {
            return Ok(false); // Nonce volé ou utilisé par mauvais utilisateur
        }

        // Marquer le nonce comme utilisé (single-use)
        nonce.used = true;

        // Supprimer le nonce du cache (single-use)
        nonces.remove(token);

        Ok(true)
    }

    /// Nettoie les nonces expirés
    ///
    /// Appelé périodiquement pour éviter l'accumulation en mémoire
    pub fn cleanup_expired(&self) {
        let mut nonces = self.nonces.write();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Supprimer tous les nonces expirés ou déjà utilisés
        nonces.retain(|_, nonce| {
            !nonce.used && nonce.expires_at > now
        });
    }

    /// Révoque tous les nonces d'un utilisateur
    ///
    /// Utile lors d'un logout ou d'un changement de session
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur dont révoquer les nonces
    pub fn revoke_user_nonces(&self, username: &str) {
        let mut nonces = self.nonces.write();
        nonces.retain(|_, nonce| nonce.username != username);
    }

    /// Compte le nombre de nonces actifs (debug/monitoring)
    pub fn count_active_nonces(&self) -> usize {
        let nonces = self.nonces.read();
        nonces.len()
    }

    /// Compte le nombre de nonces actifs pour un utilisateur
    ///
    /// # Arguments
    /// * `username` - Nom d'utilisateur
    pub fn count_user_nonces(&self, username: &str) -> usize {
        let nonces = self.nonces.read();
        nonces.values()
            .filter(|n| n.username == username && n.is_valid())
            .count()
    }
}

impl Default for CsrfManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_nonce_creation() {
        let nonce = CsrfNonce::new("testuser".to_string());

        assert!(!nonce.token.is_empty());
        assert_eq!(nonce.username, "testuser");
        assert!(!nonce.used);
        assert!(nonce.is_valid());
    }

    #[test]
    fn test_nonce_expiration() {
        let mut nonce = CsrfNonce::new("testuser".to_string());

        // Simuler expiration en modifiant expires_at
        nonce.expires_at = time::OffsetDateTime::now_utc().unix_timestamp() - 1;

        assert!(nonce.is_expired());
        assert!(!nonce.is_valid());
    }

    #[test]
    fn test_generate_and_verify() {
        let manager = CsrfManager::new();
        let token = manager.generate_nonce("alice".to_string());

        // Vérifier avec le bon utilisateur
        let result = manager.verify_and_consume(&token, "alice")
            .expect("Failed to verify nonce");
        assert!(result);

        // Vérifier que le nonce est consommé (single-use)
        let result2 = manager.verify_and_consume(&token, "alice")
            .expect("Failed to verify nonce");
        assert!(!result2);
    }

    #[test]
    fn test_wrong_username() {
        let manager = CsrfManager::new();
        let token = manager.generate_nonce("alice".to_string());

        // Tenter de vérifier avec un autre utilisateur
        let result = manager.verify_and_consume(&token, "bob")
            .expect("Failed to verify nonce");
        assert!(!result);
    }

    #[test]
    fn test_cleanup_expired() {
        let manager = CsrfManager::new();

        // Générer plusieurs nonces
        let _token1 = manager.generate_nonce("alice".to_string());
        let _token2 = manager.generate_nonce("bob".to_string());

        assert_eq!(manager.count_active_nonces(), 2);

        // Simuler expiration en modifiant directement les nonces
        {
            let mut nonces = manager.nonces.write();
            let expired_time = time::OffsetDateTime::now_utc().unix_timestamp() - 1;
            for nonce in nonces.values_mut() {
                nonce.expires_at = expired_time;
            }
        }

        // Nettoyer les nonces expirés
        manager.cleanup_expired();

        assert_eq!(manager.count_active_nonces(), 0);
    }

    #[test]
    fn test_revoke_user_nonces() {
        let manager = CsrfManager::new();

        let _token1 = manager.generate_nonce("alice".to_string());
        let _token2 = manager.generate_nonce("alice".to_string());
        let _token3 = manager.generate_nonce("bob".to_string());

        assert_eq!(manager.count_active_nonces(), 3);
        assert_eq!(manager.count_user_nonces("alice"), 2);

        // Révoquer tous les nonces d'Alice
        manager.revoke_user_nonces("alice");

        assert_eq!(manager.count_active_nonces(), 1);
        assert_eq!(manager.count_user_nonces("alice"), 0);
        assert_eq!(manager.count_user_nonces("bob"), 1);
    }

    #[test]
    fn test_invalid_token() {
        let manager = CsrfManager::new();

        // Tester avec un token qui n'existe pas
        let result = manager.verify_and_consume("invalid-token-uuid", "alice")
            .expect("Failed to verify nonce");
        assert!(!result);
    }
}
