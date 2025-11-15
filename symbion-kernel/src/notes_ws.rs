/**
 * WebSocket Notes Streaming - Symbion Kernel
 *
 * Permet le chargement progressif des notes via WebSocket.
 * Chaque note est envoyée dès qu'elle arrive via MQTT, permettant
 * un affichage temps réel dans le frontend.
 */

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::notes_bridge::{NoteResponse, SharedNotesBridge};
use crate::http::AppState;

/// Handler WebSocket pour streaming des notes
pub async fn notes_stream_handler(
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> Response {
    // Vérifier l'API key passée en query parameter (WebSockets ne supportent pas headers custom)
    let expected_key = std::env::var("SYMBION_API_KEY").unwrap_or_default();
    let provided_key = params.get("api_key").map(|s| s.as_str()).unwrap_or("");

    if expected_key.is_empty() || provided_key != expected_key {
        eprintln!("[notes-ws] Invalid or missing API key in query parameter");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Vérifier que notes_bridge est disponible
    match app_state.notes_bridge {
        Some(notes_bridge) => {
            ws.on_upgrade(move |socket| handle_notes_stream(socket, notes_bridge))
        }
        None => {
            eprintln!("[notes-ws] Notes plugin not available");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

/// Gère la connexion WebSocket pour streamer les notes
async fn handle_notes_stream(socket: WebSocket, notes_bridge: SharedNotesBridge) {
    let (mut sender, mut receiver) = socket.split();

    eprintln!("[notes-ws] WebSocket connection established");

    // Lire les messages entrants (filtres, etc.)
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                eprintln!("[notes-ws] Received request: {}", text);

                // Parser la requête (filters optionnels)
                let filters: Option<serde_json::Value> = serde_json::from_str(&text).ok();

                // Générer un request_id unique
                let request_id = Uuid::new_v4().to_string();

                // Créer un channel pour recevoir les notes en streaming
                let (tx, mut rx) = tokio::sync::mpsc::channel::<NoteResponse>(100);

                // S'abonner aux réponses MQTT
                notes_bridge.pending_requests.lock().insert(request_id.clone(), tx);

                // Envoyer la commande MQTT pour lister les notes
                let command = json!({
                    "action": "list",
                    "request_id": request_id,
                    "filters": filters
                });

                if let Ok(cmd_json) = serde_json::to_string(&command) {
                    if let Err(e) = notes_bridge
                        .mqtt_client
                        .publish(
                            "symbion/notes/command@v1",
                            rumqttc::QoS::AtLeastOnce,
                            false,
                            cmd_json,
                        )
                        .await
                    {
                        eprintln!("[notes-ws] Failed to send MQTT command: {}", e);
                        let _ = sender
                            .send(Message::Text(
                                json!({"type": "error", "error": "Failed to send command"})
                                    .to_string()
                                    .into(),
                            ))
                            .await;
                        continue;
                    }
                }

                // Streamer les notes au fur et à mesure qu'elles arrivent via MQTT
                let mut note_count = 0;
                while let Some(response) = rx.recv().await {
                    match response {
                        NoteResponse::NoteItem { note, .. } => {
                            // Délai artificiel pour effet visuel de streaming progressif
                            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                            // Envoyer la note au frontend
                            note_count += 1;
                            let msg = json!({
                                "type": "note",
                                "note": note
                            });

                            if let Err(e) = sender.send(Message::Text(msg.to_string().into())).await {
                                eprintln!("[notes-ws] Failed to send note: {}", e);
                                break;
                            }
                        }
                        NoteResponse::ListEnd { total_count, .. } => {
                            // Envoyer le message de fin
                            let msg = json!({
                                "type": "end",
                                "total_count": total_count,
                                "received_count": note_count
                            });

                            if let Err(e) = sender.send(Message::Text(msg.to_string().into())).await {
                                eprintln!("[notes-ws] Failed to send end marker: {}", e);
                            }

                            eprintln!(
                                "[notes-ws] Stream completed: {}/{} notes",
                                note_count, total_count
                            );
                            break;
                        }
                        NoteResponse::Error { error, .. } => {
                            let msg = json!({
                                "type": "error",
                                "error": error
                            });

                            if let Err(e) = sender.send(Message::Text(msg.to_string().into())).await {
                                eprintln!("[notes-ws] Failed to send error: {}", e);
                            }
                            break;
                        }
                        _ => {}
                    }
                }

                // Nettoyer la pending request
                notes_bridge.pending_requests.lock().remove(&request_id);
            }
            Ok(Message::Close(_)) => {
                eprintln!("[notes-ws] Client closed connection");
                break;
            }
            Ok(Message::Ping(ping)) => {
                // Répondre aux pings
                if let Err(e) = sender.send(Message::Pong(ping)).await {
                    eprintln!("[notes-ws] Failed to send pong: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("[notes-ws] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    eprintln!("[notes-ws] WebSocket connection closed");
}
