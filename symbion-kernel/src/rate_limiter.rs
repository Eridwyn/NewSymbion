/**
 * RATE LIMITER GLOBAL - Protection IP-based contre les abus
 *
 * Middleware Tower pour limiter les requêtes par IP source.
 * Extrait l'IP depuis X-Forwarded-For/X-Real-IP (reverse proxy)
 * ou utilise une clé par défaut pour les connexions directes.
 *
 * Configuration :
 * - 120 requêtes/minute par IP (routes API)
 * - Exempté : /health, /metrics, /ca-certificate
 * - Nettoyage automatique des entrées expirées
 */

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Configuration du rate limiter
const REQUESTS_PER_WINDOW: usize = 300;
const WINDOW_SECONDS: u64 = 60;
const CLEANUP_INTERVAL: usize = 500; // Cleanup toutes les 500 requêtes

/// Store partagé des requêtes par IP
#[derive(Clone)]
pub struct RateLimitStore {
    entries: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    request_counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            request_counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Vérifie si une IP a dépassé la limite
    fn check_and_record(&self, ip: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut entries = self.entries.lock();

        let timestamps = entries.entry(ip.to_string()).or_insert_with(Vec::new);

        // Supprimer les entrées hors fenêtre
        timestamps.retain(|t| now.duration_since(*t).as_secs() < WINDOW_SECONDS);

        if timestamps.len() >= REQUESTS_PER_WINDOW {
            // Calculer le temps restant avant expiration de la plus ancienne entrée
            if let Some(oldest) = timestamps.first() {
                let elapsed = now.duration_since(*oldest).as_secs();
                let retry_after = WINDOW_SECONDS.saturating_sub(elapsed);
                return Err(retry_after);
            }
            return Err(WINDOW_SECONDS);
        }

        timestamps.push(now);

        // Cleanup périodique des IPs inactives
        let count = self.request_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % CLEANUP_INTERVAL == 0 {
            entries.retain(|_, ts| {
                ts.last()
                    .map(|t| now.duration_since(*t).as_secs() < WINDOW_SECONDS * 2)
                    .unwrap_or(false)
            });
        }

        Ok(())
    }
}

/// Extrait l'IP du client depuis les headers proxy ou utilise un fallback
/// Ordre de priorité : CF-Connecting-IP (Cloudflare) > X-Real-IP (nginx) > X-Forwarded-For > fallback
fn extract_client_ip(req: &Request) -> String {
    // 1. CF-Connecting-IP (Cloudflare proxy — source de vérité pour symbion.markcha.fr)
    if let Some(cf_ip) = req.headers().get("cf-connecting-ip") {
        if let Ok(ip) = cf_ip.to_str() {
            let ip = ip.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    // 2. X-Real-IP (nginx reverse proxy local)
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            let ip = ip.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    // 3. X-Forwarded-For (standard proxy — première IP = client original)
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first_ip) = xff_str.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    // 4. Fallback : connexion directe (localhost/LAN)
    "direct".to_string()
}

/// Middleware de rate limiting global
pub async fn rate_limit_middleware(
    axum::extract::State(app): axum::extract::State<crate::http::AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let path = req.uri().path();

    // Exempter les routes de monitoring
    if path.starts_with("/health")
        || path.starts_with("/metrics")
        || path == "/ca-certificate"
    {
        return Ok(next.run(req).await);
    }

    let client_ip = extract_client_ip(&req);

    // Exempter les connexions directes (localhost/LAN sans proxy)
    // Le rate limiting protège uniquement les accès via proxy externe (Cloudflare/nginx)
    if client_ip == "direct" {
        return Ok(next.run(req).await);
    }

    if let Err(retry_after) = app.rate_limiter.check_and_record(&client_ip) {
        eprintln!(
            "[rate-limit] 🚫 IP {} rate limited on {} (retry in {}s)",
            client_ip, path, retry_after
        );

        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", retry_after.to_string())],
            "Too many requests. Please slow down.",
        )
            .into_response());
    }

    Ok(next.run(req).await)
}
