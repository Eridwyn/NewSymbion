/**
 * Bootstrap: Authentication & Security Subsystem
 *
 * Initializes: AuthManager, MfaManager, CsrfManager, WebAuthnManager, DeviceTrustManager
 */

use crate::auth::AuthManager;
use crate::csrf::CsrfManager;
use crate::device_trust::DeviceTrustManager;
use crate::mfa::MfaManager;
use crate::webauthn::WebAuthnManager;
use std::sync::Arc;

pub struct AuthSubsystem {
    pub auth_manager: AuthManager,
    pub mfa_manager: Arc<MfaManager>,
    pub csrf_manager: Arc<CsrfManager>,
    pub webauthn_manager: WebAuthnManager,
    pub device_trust_manager: DeviceTrustManager,
}

/// Validate required secrets exist. Exits process if missing.
pub fn validate_required_secrets() {
    let mut missing = Vec::new();

    if std::env::var("SYMBION_JWT_SECRET").is_err() {
        missing.push("SYMBION_JWT_SECRET");
    }
    if std::env::var("SYMBION_API_KEY").is_err() {
        missing.push("SYMBION_API_KEY");
    }

    if !missing.is_empty() {
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║ 🔴 SECURITY: Missing required environment variables            ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        for var in &missing {
            eprintln!("║   ❌ {}                                       ║", var);
        }
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ The kernel CANNOT start without these secrets configured.      ║");
        eprintln!("║ See .env.example for required configuration.                   ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
        std::process::exit(1);
    }

    println!("[SECURITY] All required secrets validated ✓");
}

pub fn init_auth() -> AuthSubsystem {
    let auth_manager = match AuthManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("[kernel] failed to initialize auth manager: {}", e);
            std::process::exit(1);
        }
    };

    let mfa_manager = Arc::new(MfaManager::new(
        "Symbion".to_string(),
        "Symbion".to_string(),
    ));
    println!("[kernel] initialized MFA manager");

    let csrf_manager = Arc::new(CsrfManager::new());
    println!("[kernel] initialized CSRF manager");

    let rp_id = std::env::var("SYMBION_WEBAUTHN_RP_ID")
        .unwrap_or_else(|_| "symbion.local".to_string());
    let rp_origin = std::env::var("SYMBION_WEBAUTHN_RP_ORIGIN")
        .unwrap_or_else(|_| "https://symbion.local:3000".to_string());
    let webauthn_storage_path = std::path::PathBuf::from("./data/webauthn_credentials.json");
    let webauthn_manager = match WebAuthnManager::new(&rp_id, &rp_origin, webauthn_storage_path) {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("[kernel] failed to initialize webauthn manager: {}", e);
            std::process::exit(1);
        }
    };
    println!("[kernel] initialized WebAuthn manager");

    let device_trust_manager = match DeviceTrustManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("[kernel] failed to initialize device trust manager: {}", e);
            std::process::exit(1);
        }
    };
    println!("[kernel] initialized Device Trust manager");

    AuthSubsystem {
        auth_manager,
        mfa_manager,
        csrf_manager,
        webauthn_manager,
        device_trust_manager,
    }
}
