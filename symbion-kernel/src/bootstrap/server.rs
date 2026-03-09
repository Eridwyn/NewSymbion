/**
 * Bootstrap: HTTP/HTTPS Server
 *
 * Builds AppState, configures TLS, starts HTTPS + HTTP redirect servers,
 * and handles graceful shutdown.
 */

use crate::http::AppState;
use std::net::SocketAddr;

pub async fn run_servers(app_state: AppState, boot_start: std::time::Instant) {
    let app_https = crate::http::build_router(app_state.clone());

    // TLS certificates
    let cert_path = std::env::var("SYMBION_TLS_CERT_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/cert-mkcert.pem".to_string());
    let key_path = std::env::var("SYMBION_TLS_KEY_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/key-mkcert.pem".to_string());

    let https_port: u16 = std::env::var("SYMBION_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);

    let http_port: u16 = std::env::var("SYMBION_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let https_addr = SocketAddr::from(([0, 0, 0, 0], https_port));
    let http_addr = SocketAddr::from(([0, 0, 0, 0], http_port));

    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .expect(&format!("Failed to load TLS certificates from {} and {}", cert_path, key_path));

    eprintln!("[kernel] ⏱ boot complete in {:?}", boot_start.elapsed());
    println!("[kernel] 🔒 HTTPS enabled - listening on https://{}", https_addr);
    println!("[kernel] TLS cert: {}", cert_path);
    println!("[kernel] TLS key: {}", key_path);

    let redirect_app = crate::http::build_redirect_router(https_port);
    println!("[kernel] 🔄 HTTP redirect enabled - listening on http://{} → https://localhost:{}", http_addr, https_port);

    // Launch both servers with graceful shutdown
    let https_server = axum_server::bind_rustls(https_addr, tls_config)
        .serve(app_https.into_make_service());

    let http_listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .expect(&format!("[kernel] FATAL: cannot bind HTTP port {} — check if already in use", http_port));

    let http_server = axum::serve(http_listener, redirect_app.into_make_service());

    let shutdown = async {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        #[cfg(unix)]
        let sigterm_recv = sigterm.recv();
        #[cfg(not(unix))]
        let sigterm_recv = std::future::pending::<Option<()>>();

        tokio::select! {
            _ = ctrl_c => eprintln!("[kernel] SIGINT received, shutting down gracefully..."),
            _ = sigterm_recv => eprintln!("[kernel] SIGTERM received, shutting down gracefully..."),
        }
    };

    tokio::select! {
        res = https_server => {
            if let Err(e) = res { eprintln!("[kernel] HTTPS server error: {}", e); }
        }
        res = http_server => {
            if let Err(e) = res { eprintln!("[kernel] HTTP server error: {}", e); }
        }
        _ = shutdown => {
            eprintln!("[kernel] Graceful shutdown complete");
        }
    }
}
