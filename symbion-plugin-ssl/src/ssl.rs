//! SSL certificate checking logic

use chrono::{DateTime, Utc};
use native_tls::TlsConnector;
use sha2::{Sha256, Digest};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use x509_parser::prelude::*;

/// SSL certificate status
#[derive(Debug, Clone)]
pub struct CertificateStatus {
    /// Domain checked
    pub hostname: String,

    /// Port checked
    pub port: u16,

    /// Whether the certificate is valid
    pub valid: bool,

    /// Certificate expiry date (UTC)
    pub expiry_date: Option<DateTime<Utc>>,

    /// Days until expiry
    pub days_remaining: Option<i64>,

    /// Certificate issuer (e.g., "Let's Encrypt")
    pub issuer: Option<String>,

    /// Certificate subject CN
    pub subject: Option<String>,

    /// Certificate fingerprint (SHA256 hex)
    pub fingerprint: Option<String>,

    /// Error message if check failed
    pub error: Option<String>,

    /// Check timestamp
    pub checked_at: DateTime<Utc>,
}

/// SSL certificate checker
pub struct SslChecker {
    /// Connection timeout
    timeout: Duration,
}

impl SslChecker {
    /// Create new SSL checker with default timeout
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Check SSL certificate for a domain
    pub fn check(&self, hostname: &str, port: u16) -> CertificateStatus {
        let checked_at = Utc::now();
        let address = format!("{}:{}", hostname, port);

        // Helper to create error status
        let error_status = |error: String| CertificateStatus {
            hostname: hostname.to_string(),
            port,
            valid: false,
            expiry_date: None,
            days_remaining: None,
            issuer: None,
            subject: None,
            fingerprint: None,
            error: Some(error),
            checked_at,
        };

        // Resolve hostname to socket address
        let socket_addr = match address.to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr,
                None => return error_status("DNS resolution returned no addresses".to_string()),
            },
            Err(e) => return error_status(format!("DNS resolution failed: {}", e)),
        };

        // Attempt TCP connection with timeout
        let stream = match TcpStream::connect_timeout(&socket_addr, self.timeout) {
            Ok(s) => s,
            Err(e) => return error_status(format!("TCP connection failed: {}", e)),
        };

        // Set read/write timeouts
        let _ = stream.set_read_timeout(Some(self.timeout));
        let _ = stream.set_write_timeout(Some(self.timeout));

        // Build TLS connector (accept any cert to get the chain)
        let connector = match TlsConnector::builder()
            .danger_accept_invalid_certs(true) // We want to inspect even invalid certs
            .build()
        {
            Ok(c) => c,
            Err(e) => return error_status(format!("TLS connector error: {}", e)),
        };

        // Perform TLS handshake
        let tls_stream = match connector.connect(hostname, stream) {
            Ok(s) => s,
            Err(e) => return error_status(format!("TLS handshake failed: {}", e)),
        };

        // Get peer certificate
        let cert_der = match tls_stream.peer_certificate() {
            Ok(Some(cert)) => cert.to_der().unwrap_or_default(),
            Ok(None) => return error_status("No peer certificate".to_string()),
            Err(e) => return error_status(format!("Failed to get peer cert: {}", e)),
        };

        // Calculate fingerprint (SHA256)
        let fingerprint = {
            let mut hasher = Sha256::new();
            hasher.update(&cert_der);
            let result = hasher.finalize();
            hex::encode(result)
        };

        // Parse X509 certificate
        match X509Certificate::from_der(&cert_der) {
            Ok((_, cert)) => {
                // Extract expiry date
                let not_after = cert.validity().not_after;
                let expiry_timestamp = not_after.timestamp();
                let expiry_date = DateTime::from_timestamp(expiry_timestamp, 0)
                    .unwrap_or_else(|| Utc::now());

                // Calculate days remaining
                let now = Utc::now();
                let duration = expiry_date.signed_duration_since(now);
                let days_remaining = duration.num_days();

                // Extract issuer
                let issuer = cert.issuer()
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .map(|s| s.to_string());

                // Extract subject
                let subject = cert.subject()
                    .iter_common_name()
                    .next()
                    .and_then(|cn| cn.as_str().ok())
                    .map(|s| s.to_string());

                // Determine validity
                let valid = days_remaining > 0 && cert.validity().is_valid();

                CertificateStatus {
                    hostname: hostname.to_string(),
                    port,
                    valid,
                    expiry_date: Some(expiry_date),
                    days_remaining: Some(days_remaining),
                    issuer,
                    subject,
                    fingerprint: Some(fingerprint),
                    error: None,
                    checked_at,
                }
            }
            Err(e) => {
                CertificateStatus {
                    hostname: hostname.to_string(),
                    port,
                    valid: false,
                    expiry_date: None,
                    days_remaining: None,
                    issuer: None,
                    subject: None,
                    fingerprint: Some(fingerprint), // Still provide fingerprint even if parse fails
                    error: Some(format!("X509 parse error: {:?}", e)),
                    checked_at,
                }
            }
        }
    }

    /// Check if domain is online (ping via TCP connect)
    pub fn check_online(&self, hostname: &str, port: u16) -> bool {
        let address = format!("{}:{}", hostname, port);

        // DNS resolution attempt
        if let Ok(mut addrs) = address.to_socket_addrs() {
            if let Some(addr) = addrs.next() {
                return TcpStream::connect_timeout(&addr, Duration::from_secs(5)).is_ok();
            }
        }
        false
    }
}

impl Default for SslChecker {
    fn default() -> Self {
        Self::new()
    }
}
