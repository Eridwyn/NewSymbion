use anyhow::{Context, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
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
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: String,
    pub expires_at: i64,
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
                password_hash: hash("Sourire951", DEFAULT_COST)
                    .context("Failed to hash default password")?,
                role: "admin".to_string(),
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
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

    pub fn authenticate(&self, username: &str, password: &str) -> Result<LoginResponse> {
        // Check rate limit BEFORE doing anything else
        self.check_rate_limit(username)?;

        // Record attempt immediately (even if user doesn't exist)
        // This prevents brute-force attacks with non-existent usernames
        self.record_attempt(username);

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

        // Generate JWT
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + (get_token_expiry_hours() * 3600);

        let claims = Claims {
            sub: user.username.clone(),
            role: user.role.clone(),
            exp: expires_at,
            iat: now,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .context("Failed to generate JWT token")?;

        println!("[auth] User '{}' authenticated successfully", username);

        Ok(LoginResponse {
            token,
            username: user.username.clone(),
            role: user.role.clone(),
            expires_at,
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

    pub fn create_user(&self, username: &str, password: &str, role: &str) -> Result<()> {
        let mut users = self.users.write();

        if users.contains_key(username) {
            anyhow::bail!("User '{}' already exists", username);
        }

        let password_hash = hash(password, DEFAULT_COST)
            .context("Failed to hash password")?;

        let user = User {
            username: username.to_string(),
            password_hash,
            role: role.to_string(),
            created_at: OffsetDateTime::now_utc().unix_timestamp(),
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
}
