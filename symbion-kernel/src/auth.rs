use anyhow::{Context, Result};
use bcrypt::{hash, verify};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use time::OffsetDateTime;

const USERS_FILE: &str = "users.json";

// Rate Limiting Configuration
const MAX_LOGIN_ATTEMPTS: usize = 5;
const RATE_LIMIT_WINDOW_SECS: i64 = 900; // 15 minutes

fn get_jwt_secret() -> String {
    std::env::var("SYMBION_JWT_SECRET")
        .expect("SYMBION_JWT_SECRET must be set in environment (see .env.example)")
}

fn get_token_expiry_hours() -> i64 {
    std::env::var("SYMBION_TOKEN_EXPIRY_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8) // Default: 8 hours
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: i64,
    /// Configuration MFA (optionnelle, None si MFA non activée)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mfa_config: Option<crate::mfa::MfaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // username
    pub role: String,
    pub exp: i64,         // expiry timestamp
    pub iat: i64,         // issued at timestamp
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub totp_code: Option<String>,
    #[serde(default)]
    pub remember_device: bool,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: String,
    pub expires_at: i64,
    /// Indique si MFA est requis pour ce compte
    pub requires_mfa: bool,
    /// Device token pour bypass MFA (optionnel, renvoyé si remember_device=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub username: String,
    pub role: String,
    pub expires_at: i64,
}

#[derive(Clone)]
pub struct AuthManager {
    users: Arc<RwLock<HashMap<String, User>>>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    // Rate limiting: username -> Vec of attempt timestamps
    login_attempts: Arc<RwLock<HashMap<String, Vec<i64>>>>,
}

impl AuthManager {
    pub fn new() -> Result<Self> {
        let users = Self::load_users()?;
        let jwt_secret = get_jwt_secret();

        Ok(Self {
            users: Arc::new(RwLock::new(users)),
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn load_users() -> Result<HashMap<String, User>> {
        if !Path::new(USERS_FILE).exists() {
            // Create default admin user: Mark / Sourire951
            println!("[auth] Creating default admin user...");
            let default_user = User {
                username: "Mark".to_string(),
                password_hash: hash("Sourire951", 12)
                    .context("Failed to hash default password")?,
                role: "admin".to_string(),
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
                mfa_config: None,
            };

            let mut users = HashMap::new();
            users.insert(default_user.username.clone(), default_user);

            // Save to file
            let json = serde_json::to_string_pretty(&users)
                .context("Failed to serialize default users")?;
            fs::write(USERS_FILE, json)
                .context("Failed to write default users file")?;

            println!("[auth] Default user 'Mark' created");
            return Ok(users);
        }

        let content = fs::read_to_string(USERS_FILE)
            .context("Failed to read users file")?;
        let users: HashMap<String, User> = serde_json::from_str(&content)
            .context("Failed to parse users file")?;

        println!("[auth] Loaded {} user(s)", users.len());
        Ok(users)
    }

    /// Save users to users.json file
    fn save_users(&self, users: &HashMap<String, User>) -> Result<()> {
        let json = serde_json::to_string_pretty(users)
            .context("Failed to serialize users")?;
        fs::write(USERS_FILE, json)
            .context("Failed to write users file")?;
        Ok(())
    }

    /// Check if user has exceeded rate limit
    /// Returns Ok(()) if allowed, Err with remaining time if blocked
    fn check_rate_limit(&self, username: &str) -> Result<()> {
        let mut attempts = self.login_attempts.write();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        // Get attempts for this username
        let user_attempts = attempts.entry(username.to_string()).or_insert_with(Vec::new);

        // Remove old attempts (outside the 15-minute window)
        user_attempts.retain(|&timestamp| now - timestamp < RATE_LIMIT_WINDOW_SECS);

        // Check if limit exceeded
        if user_attempts.len() >= MAX_LOGIN_ATTEMPTS {
            let oldest_attempt = user_attempts.first().unwrap();
            let wait_until = oldest_attempt + RATE_LIMIT_WINDOW_SECS;
            let wait_seconds = wait_until - now;
            let wait_minutes = (wait_seconds + 59) / 60; // Round up

            anyhow::bail!(
                "Too many login attempts. Please wait {} minute(s) before trying again.",
                wait_minutes
            );
        }

        Ok(())
    }

    /// Record a login attempt (successful or failed)
    fn record_attempt(&self, username: &str) {
        let mut attempts = self.login_attempts.write();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        let user_attempts = attempts.entry(username.to_string()).or_insert_with(Vec::new);
        user_attempts.push(now);

        println!("[auth] Login attempt recorded for '{}' ({} attempts in window)",
                 username, user_attempts.len());
    }

    pub fn authenticate(&self, username: &str, password: &str, totp_code: Option<&str>, trusted_device: bool) -> Result<LoginResponse> {
        // Check rate limit BEFORE doing anything else
        self.check_rate_limit(username)?;

        // Record attempt immediately (even if user doesn't exist)
        // This prevents brute-force attacks with non-existent usernames
        self.record_attempt(username);

        // Clone user data we'll need later (before any lock drops)
        let (user_username, user_role, requires_mfa) = {
            let users = self.users.read();

            let user = users
                .get(username)
                .context("Invalid username or password")?;

            // Verify password
            let password_valid = verify(password, &user.password_hash)
                .context("Password verification failed")?;

            if !password_valid {
                anyhow::bail!("Invalid username or password");
            }

            // Clone data we'll need after releasing lock
            let requires_mfa = user.mfa_config
                .as_ref()
                .map(|config| config.enabled)
                .unwrap_or(false);

            (user.username.clone(), user.role.clone(), requires_mfa)
        }; // Read lock released here

        // Si MFA activé, vérifier le code TOTP ou backup code (sauf si device de confiance)
        if requires_mfa {
            if !trusted_device {
                // Device non-trusted: vérifier le code TOTP ou backup code
                let totp_code = totp_code.ok_or_else(|| {
                    anyhow::anyhow!("MFA is enabled. Please provide a TOTP code.")
                })?;

                // Check backup codes first (single-use)
                let is_backup_code = {
                    let users = self.users.read();
                    users.get(username)
                        .and_then(|u| u.mfa_config.as_ref())
                        .map(|mfa| mfa.backup_codes.contains(&totp_code.to_string()))
                        .unwrap_or(false)
                };

                if is_backup_code {
                    // Backup code trouvé : le retirer et sauvegarder
                    let mut users_write = self.users.write();
                    if let Some(user_mut) = users_write.get_mut(username) {
                        if let Some(ref mut mfa_mut) = user_mut.mfa_config {
                            if let Some(index) = mfa_mut.backup_codes.iter().position(|c| c == totp_code) {
                                mfa_mut.backup_codes.remove(index);
                                mfa_mut.last_verified_at = time::OffsetDateTime::now_utc().unix_timestamp();

                                let remaining = mfa_mut.backup_codes.len();

                                // Sauvegarder users.json (clone pour éviter conflit borrow)
                                drop(user_mut); // Release mutable borrow
                                let users_clone = users_write.clone();
                                drop(users_write); // Release write lock

                                if let Err(e) = self.save_users(&users_clone) {
                                    eprintln!("[auth] Failed to save users after backup code consumption: {}", e);
                                }

                                println!("[auth] User '{}' authenticated with MFA backup code (remaining: {})",
                                    username, remaining);
                            }
                        }
                    }
                } else {
                    // Pas un backup code : vérifier le code TOTP
                    let mfa_manager = crate::mfa::MfaManager::new("Symbion".to_string(), "Symbion IoT".to_string());

                    let secret = {
                        let users = self.users.read();
                        users.get(username)
                            .and_then(|u| u.mfa_config.as_ref())
                            .map(|mfa| mfa.secret_base32.clone())
                            .context("MFA config not found")?
                    };

                    let is_valid = mfa_manager.verify_totp_with_secret(&secret, totp_code)
                        .context("Failed to verify TOTP code")?;

                    if !is_valid {
                        anyhow::bail!("Invalid TOTP code");
                    }

                    println!("[auth] User '{}' authenticated with MFA TOTP successfully", username);
                }
            } else {
                // Device de confiance: bypass MFA
                println!("[auth] User '{}' authenticated with MFA bypassed (trusted device)", username);
            }
        } else {
            println!("[auth] User '{}' authenticated successfully (no MFA)", username);
        }

        // Generate JWT après vérification MFA
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + (get_token_expiry_hours() * 3600);

        let claims = Claims {
            sub: user_username.clone(),
            role: user_role.clone(),
            exp: expires_at,
            iat: now,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .context("Failed to generate JWT token")?;

        Ok(LoginResponse {
            token,
            username: user_username,
            role: user_role,
            expires_at,
            requires_mfa,
            device_token: None, // Sera rempli par http.rs si remember_device=true
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )
        .context("Invalid or expired token")?;

        Ok(token_data.claims)
    }

    pub fn get_session_info(&self, token: &str) -> Result<SessionInfo> {
        let claims = self.verify_token(token)?;

        Ok(SessionInfo {
            username: claims.sub,
            role: claims.role,
            expires_at: claims.exp,
        })
    }

    /// Create JWT token for authenticated user (for WebAuthn, etc.)
    pub fn create_token_for_user(&self, username: &str) -> Result<String> {
        let (user_role, user_username) = {
            let users = self.users.read();
            let user = users
                .get(username)
                .context("User not found")?;
            (user.role.clone(), user.username.clone())
        };

        let now = OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + (get_token_expiry_hours() * 3600);

        let claims = Claims {
            sub: user_username,
            role: user_role,
            exp: expires_at,
            iat: now,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .context("Failed to generate JWT token")?;

        Ok(token)
    }

    pub fn create_user(&self, username: &str, password: &str, role: &str) -> Result<()> {
        let mut users = self.users.write();

        if users.contains_key(username) {
            anyhow::bail!("User '{}' already exists", username);
        }

        let password_hash = hash(password, 12)
            .context("Failed to hash password")?;

        let user = User {
            username: username.to_string(),
            password_hash,
            role: role.to_string(),
            created_at: OffsetDateTime::now_utc().unix_timestamp(),
            mfa_config: None,
        };

        users.insert(username.to_string(), user);

        // Save to file
        let json = serde_json::to_string_pretty(&*users)
            .context("Failed to serialize users")?;
        fs::write(USERS_FILE, json)
            .context("Failed to write users file")?;

        println!("[auth] User '{}' created with role '{}'", username, role);
        Ok(())
    }

    /// Récupère un utilisateur par son nom
    pub fn get_user(&self, username: &str) -> Option<User> {
        let users = self.users.read();
        users.get(username).cloned()
    }

    /// Met à jour la configuration MFA d'un utilisateur
    pub fn update_user_mfa(&self, username: &str, mfa_config: Option<crate::mfa::MfaConfig>) -> Result<()> {
        let mut users = self.users.write();

        let user = users.get_mut(username)
            .context(format!("User '{}' not found", username))?;

        user.mfa_config = mfa_config;

        // Sauvegarder dans le fichier
        let json = serde_json::to_string_pretty(&*users)
            .context("Failed to serialize users")?;
        fs::write(USERS_FILE, json)
            .context("Failed to write users file")?;

        println!("[auth] MFA config updated for user '{}'", username);
        Ok(())
    }

    /// Vérifie le mot de passe d'un utilisateur (sans rate limiting)
    pub fn verify_password(&self, username: &str, password: &str) -> Result<bool> {
        let users = self.users.read();

        let user = users.get(username)
            .context(format!("User '{}' not found", username))?;

        let valid = verify(password, &user.password_hash)
            .context("Failed to verify password")?;

        Ok(valid)
    }

    /// Met à jour le mot de passe d'un utilisateur
    pub fn update_password(&self, username: &str, new_password: &str) -> Result<()> {
        let mut users = self.users.write();

        let user = users.get_mut(username)
            .context(format!("User '{}' not found", username))?;

        // Hash du nouveau mot de passe
        let password_hash = hash(new_password, 12)
            .context("Failed to hash password")?;

        user.password_hash = password_hash;

        // Sauvegarder dans le fichier
        let json = serde_json::to_string_pretty(&*users)
            .context("Failed to serialize users")?;
        fs::write(USERS_FILE, json)
            .context("Failed to write users file")?;

        println!("[auth] Password updated for user '{}'", username);
        Ok(())
    }

    /// Recharge les utilisateurs depuis users.json sans redémarrer le kernel
    pub fn reload_users(&self) -> Result<()> {
        let new_users = Self::load_users()?;

        let mut users = self.users.write();
        *users = new_users;

        println!("[auth] Users reloaded from disk ({} user(s))", users.len());
        Ok(())
    }

    /// Supprime un utilisateur
    pub fn delete_user(&self, username: &str) -> Result<()> {
        let mut users = self.users.write();

        if !users.contains_key(username) {
            anyhow::bail!("User '{}' not found", username);
        }

        users.remove(username);

        // Sauvegarder dans le fichier
        let json = serde_json::to_string_pretty(&*users)
            .context("Failed to serialize users")?;
        fs::write(USERS_FILE, json)
            .context("Failed to write users file")?;

        println!("[auth] User '{}' deleted", username);
        Ok(())
    }

    /// Liste tous les utilisateurs (sans les mots de passe)
    pub fn list_users(&self) -> Vec<serde_json::Value> {
        let users = self.users.read();
        users.iter().map(|(username, user)| {
            serde_json::json!({
                "username": username,
                "role": user.role,
                "mfa_enabled": user.mfa_config.is_some()
            })
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // Helper to clean up test users file
    fn cleanup_test_users() {
        if Path::new(USERS_FILE).exists() {
            let _ = fs::remove_file(USERS_FILE);
        }
    }

    // Helper to create test AuthManager with fresh state
    fn create_test_auth_manager() -> Result<AuthManager> {
        cleanup_test_users();
        std::env::set_var("SYMBION_JWT_SECRET", "test-secret-1234567890123456789012345678901234567890123456789012345678901234");
        AuthManager::new()
    }

    #[test]
    fn test_auth_manager_creation() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Should have default "Mark" user created
        let users = auth.list_users();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["username"], "Mark");
        assert_eq!(users[0]["role"], "admin");

        cleanup_test_users();
    }

    #[test]
    fn test_password_hashing_bcrypt_cost_12() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Create a test user and verify bcrypt cost is 12
        auth.create_user("testuser", "TestPass123", "user")
            .expect("Failed to create user");

        let user = auth.get_user("testuser").expect("User not found");

        // Bcrypt hashes start with "$2b$12$" where 12 is the cost
        assert!(user.password_hash.starts_with("$2b$12$") || user.password_hash.starts_with("$2a$12$"),
                "Password hash should use bcrypt cost 12, got: {}", user.password_hash);

        cleanup_test_users();
    }

    #[test]
    fn test_authenticate_valid_credentials() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Test with default Mark user
        let result = auth.authenticate("Mark", "Sourire951", None, false);
        assert!(result.is_ok(), "Valid credentials should authenticate");

        let response = result.unwrap();
        assert_eq!(response.username, "Mark");
        assert_eq!(response.role, "admin");
        assert!(!response.token.is_empty());
        assert!(!response.requires_mfa);

        cleanup_test_users();
    }

    #[test]
    fn test_authenticate_invalid_password() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let result = auth.authenticate("Mark", "WrongPassword", None, false);
        assert!(result.is_err(), "Invalid password should fail");
        assert!(result.unwrap_err().to_string().contains("Invalid username or password"));

        cleanup_test_users();
    }

    #[test]
    fn test_authenticate_nonexistent_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let result = auth.authenticate("NonExistentUser", "SomePassword", None, false);
        assert!(result.is_err(), "Non-existent user should fail");
        assert!(result.unwrap_err().to_string().contains("Invalid username or password"));

        cleanup_test_users();
    }

    #[test]
    fn test_jwt_token_generation() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let response = auth.authenticate("Mark", "Sourire951", None, false)
            .expect("Authentication failed");

        // Token should not be empty
        assert!(!response.token.is_empty());

        // Token should have 3 parts (header.payload.signature)
        let parts: Vec<&str> = response.token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts");

        // Expiry should be in the future (8 hours default)
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        assert!(response.expires_at > now, "Token should expire in the future");
        assert!(response.expires_at <= now + (8 * 3600 + 10), "Token should expire within ~8 hours");

        cleanup_test_users();
    }

    #[test]
    fn test_jwt_token_verification() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let response = auth.authenticate("Mark", "Sourire951", None, false)
            .expect("Authentication failed");

        // Verify the token
        let claims = auth.verify_token(&response.token)
            .expect("Token verification failed");

        assert_eq!(claims.sub, "Mark");
        assert_eq!(claims.role, "admin");
        assert!(claims.exp > claims.iat, "Expiry should be after issued time");

        cleanup_test_users();
    }

    #[test]
    fn test_jwt_token_invalid_signature() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Create a token with different secret to simulate tampering
        std::env::set_var("SYMBION_JWT_SECRET", "different-secret-key");
        let auth2 = AuthManager::new().expect("Failed to create second AuthManager");

        let token = auth2.create_token_for_user("Mark").expect("Failed to create token");

        // Try to verify with original auth manager (different secret)
        let result = auth.verify_token(&token);
        assert!(result.is_err(), "Token with wrong signature should fail verification");

        cleanup_test_users();
    }

    #[test]
    fn test_create_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let result = auth.create_user("alice", "AlicePass123", "user");
        assert!(result.is_ok(), "User creation should succeed");

        let user = auth.get_user("alice").expect("User should exist");
        assert_eq!(user.username, "alice");
        assert_eq!(user.role, "user");
        assert!(user.mfa_config.is_none());

        cleanup_test_users();
    }

    #[test]
    fn test_create_duplicate_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        auth.create_user("bob", "BobPass123", "user")
            .expect("First user creation should succeed");

        let result = auth.create_user("bob", "AnotherPass", "admin");
        assert!(result.is_err(), "Duplicate user creation should fail");
        assert!(result.unwrap_err().to_string().contains("already exists"));

        cleanup_test_users();
    }

    #[test]
    fn test_delete_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        auth.create_user("charlie", "CharliePass123", "user")
            .expect("User creation should succeed");

        assert!(auth.get_user("charlie").is_some(), "User should exist");

        let result = auth.delete_user("charlie");
        assert!(result.is_ok(), "User deletion should succeed");

        assert!(auth.get_user("charlie").is_none(), "User should not exist after deletion");

        cleanup_test_users();
    }

    #[test]
    fn test_rate_limiting() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Attempt login 5 times with wrong password (max attempts)
        for i in 0..5 {
            let result = auth.authenticate("Mark", "WrongPassword", None, false);
            assert!(result.is_err(), "Attempt {} should fail with wrong password", i + 1);
        }

        // 6th attempt should be rate limited
        let result = auth.authenticate("Mark", "WrongPassword", None, false);
        assert!(result.is_err(), "6th attempt should fail");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Too many login attempts") || error_msg.contains("wait"),
                "Error should mention rate limiting, got: {}", error_msg);

        // Even correct password should be rate limited now
        let result = auth.authenticate("Mark", "Sourire951", None, false);
        assert!(result.is_err(), "Correct password should also be rate limited");

        cleanup_test_users();
    }

    #[test]
    fn test_rate_limit_different_users() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        auth.create_user("alice", "AlicePass", "user")
            .expect("Failed to create alice");
        auth.create_user("bob", "BobPass", "user")
            .expect("Failed to create bob");

        // Rate limit alice
        for _ in 0..5 {
            let _ = auth.authenticate("alice", "WrongPassword", None, false);
        }

        // Alice should be rate limited
        let result = auth.authenticate("alice", "AlicePass", None, false);
        assert!(result.is_err(), "Alice should be rate limited");

        // Bob should still be able to login
        let result = auth.authenticate("bob", "BobPass", None, false);
        assert!(result.is_ok(), "Bob should not be rate limited");

        cleanup_test_users();
    }

    #[test]
    fn test_session_info() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let response = auth.authenticate("Mark", "Sourire951", None, false)
            .expect("Authentication failed");

        let session = auth.get_session_info(&response.token)
            .expect("Failed to get session info");

        assert_eq!(session.username, "Mark");
        assert_eq!(session.role, "admin");
        assert_eq!(session.expires_at, response.expires_at);

        cleanup_test_users();
    }

    #[test]
    fn test_create_token_for_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let token = auth.create_token_for_user("Mark")
            .expect("Failed to create token");

        assert!(!token.is_empty());

        // Verify the token is valid
        let claims = auth.verify_token(&token)
            .expect("Token verification failed");

        assert_eq!(claims.sub, "Mark");
        assert_eq!(claims.role, "admin");

        cleanup_test_users();
    }

    #[test]
    fn test_create_token_for_nonexistent_user() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let result = auth.create_token_for_user("NonExistentUser");
        assert!(result.is_err(), "Creating token for non-existent user should fail");
        assert!(result.unwrap_err().to_string().contains("User not found"));

        cleanup_test_users();
    }

    #[test]
    fn test_list_users() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        auth.create_user("alice", "AlicePass", "user")
            .expect("Failed to create alice");
        auth.create_user("bob", "BobPass", "admin")
            .expect("Failed to create bob");

        let users = auth.list_users();
        assert_eq!(users.len(), 3); // Mark + alice + bob

        // Verify no password hashes are exposed
        for user in users {
            assert!(user.get("password_hash").is_none(), "Password hash should not be exposed");
            assert!(user.get("username").is_some());
            assert!(user.get("role").is_some());
        }

        cleanup_test_users();
    }

    #[test]
    fn test_update_user_mfa() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        let mfa_config = crate::mfa::MfaConfig {
            enabled: true,
            secret_base32: "TESTSECRET123".to_string(),
            backup_codes: vec!["12345678".to_string()],
            last_verified_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            recovery_email: Some("mark@example.com".to_string()),
            setup_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        };

        let result = auth.update_user_mfa("Mark", Some(mfa_config.clone()));
        assert!(result.is_ok(), "MFA update should succeed");

        let user = auth.get_user("Mark").expect("User should exist");
        assert!(user.mfa_config.is_some());
        assert_eq!(user.mfa_config.unwrap().enabled, true);

        cleanup_test_users();
    }

    #[test]
    fn test_reload_users() {
        let auth = create_test_auth_manager().expect("Failed to create AuthManager");

        // Create a user
        auth.create_user("alice", "AlicePass", "user")
            .expect("Failed to create alice");

        // Modify users.json directly
        let mut users = HashMap::new();
        users.insert("Mark".to_string(), User {
            username: "Mark".to_string(),
            password_hash: hash("NewPassword", 12).expect("Failed to hash"),
            role: "admin".to_string(),
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            mfa_config: None,
        });
        users.insert("charlie".to_string(), User {
            username: "charlie".to_string(),
            password_hash: hash("CharliePass", 12).expect("Failed to hash"),
            role: "user".to_string(),
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            mfa_config: None,
        });

        let json = serde_json::to_string_pretty(&users).expect("Failed to serialize");
        fs::write(USERS_FILE, json).expect("Failed to write users file");

        // Reload users
        auth.reload_users().expect("Failed to reload users");

        // Alice should be gone, charlie should be present
        assert!(auth.get_user("alice").is_none(), "Alice should not exist after reload");
        assert!(auth.get_user("charlie").is_some(), "Charlie should exist after reload");

        cleanup_test_users();
    }
}
