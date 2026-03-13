use crate::state::AppState;
use serde_json::Value;
use std::process::Stdio;
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Minimum characters before we update the Telegram message
const MIN_DELTA_CHARS: usize = 30;
/// Minimum interval between Telegram message edits
const EDIT_INTERVAL: Duration = Duration::from_millis(1500);
/// Maximum Telegram message length
const MAX_MSG_LEN: usize = 4000;
/// Typing indicator interval
const TYPING_INTERVAL: Duration = Duration::from_secs(4);

const SYSTEM_PROMPT: &str = "Tu es l'assistant Symbion, accessible via Telegram. \
Réponds en français, concis, adapté au mobile. \
\
RÈGLES CRITIQUES: \
- Termine TOUJOURS la tâche entièrement en une seule réponse. \
- Ne t'arrête JAMAIS au milieu. Pas de confirmation intermédiaire. \
- Fais toutes les recherches puis exécute puis confirme le résultat. \
- Utilise curl avec -k pour ignorer les erreurs TLS localhost. \
\
BIBLIOTHÈQUE SYMBION (API REST): \
- Base URL: https://localhost:8443/v1/plugin-api/library \
- Header OBLIGATOIRE: x-api-key: s3cr3t-42 \
- La bibliothèque a des sections, nodes et templates. \
- AVANT de créer un node, TOUJOURS: \
  1. GET /templates → trouver le template_id approprié \
  2. GET /sections → trouver le section_id approprié \
  3. POST /nodes avec OBLIGATOIREMENT: title, template_id, section_ids (tableau), et fields selon le template \
- Un node SANS section_ids ne sera PAS visible dans l'interface! \
- Exemple création: curl -k -X POST .../nodes -H 'Content-Type: application/json' -H 'x-api-key: s3cr3t-42' -d '{\"title\":\"Mon item\",\"template_id\":\"...\",\"section_ids\":[\"...\"],\"fields\":{...}}' \
- Autres endpoints: GET /nodes, GET /search?q=..., GET /nodes/{id}/desk \
- Ne modifie JAMAIS les fichiers de données directement.";

/// Run Claude CLI and stream response to Telegram
pub async fn run_claude(
    state: &AppState,
    chat_id: ChatId,
    prompt: &str,
    cancel: CancellationToken,
) -> Result<(), String> {
    let session = state.get_session(chat_id.0);

    // Build command
    let mut cmd = Command::new(state.config.claude_path.as_os_str());
    cmd.arg("-p").arg(prompt);
    cmd.arg("--dangerously-skip-permissions");
    cmd.arg("--disable-slash-commands");
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--verbose");
    cmd.arg("--model").arg(&session.model);
    cmd.arg("--effort").arg(&session.effort);
    cmd.arg("--max-turns").arg("15");
    cmd.arg("--append-system-prompt").arg(SYSTEM_PROMPT);

    // Session continuity
    if let Some(ref sid) = session.session_id {
        cmd.arg("--resume").arg(sid);
    }

    cmd.current_dir(&state.config.claude_workdir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn process
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Impossible de lancer claude: {}", e))?;

    let stdout = child.stdout.take().ok_or("No stdout")?;
    let stderr = child.stderr.take();
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // Log stderr in background
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let mut err_reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = err_reader.next_line().await {
                if !line.is_empty() {
                    eprintln!("[telegram/claude-stderr] {}", line);
                }
            }
        });
    }

    // Send initial "thinking" message
    let sent = state
        .bot
        .send_message(chat_id, "⏳ Réflexion en cours...")
        .await
        .map_err(|e| format!("Erreur Telegram: {}", e))?;

    let msg_id = sent.id;
    let mut full_text = String::new();
    let mut last_edit = Instant::now();
    let mut last_edit_len: usize = 0;
    let mut last_typing = Instant::now() - TYPING_INTERVAL;
    let mut tools_used: Vec<String> = Vec::new();
    let mut captured_session_id: Option<String> = None;

    // Timeout
    let timeout = Duration::from_secs(state.config.claude_timeout_secs);
    let start = Instant::now();

    // Read stream-json events
    loop {
        // Timeout check
        if start.elapsed() > timeout {
            let _ = child.kill().await;
            let display = if full_text.is_empty() {
                "⏰ Timeout — pas de réponse.".to_string()
            } else {
                format!("{}\n\n⏰ Timeout", truncate_text(&full_text))
            };
            let _ = state.bot.edit_message_text(chat_id, msg_id, display).await;
            return Ok(());
        }

        // Send typing indicator periodically
        let now = Instant::now();
        if now.duration_since(last_typing) >= TYPING_INTERVAL {
            let _ = state.bot.send_chat_action(chat_id, ChatAction::Typing).await;
            last_typing = now;
        }

        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                let display = if full_text.is_empty() {
                    "❌ Annulé".to_string()
                } else {
                    format!("{}\n\n❌ Annulé", truncate_text(&full_text))
                };
                let _ = state.bot.edit_message_text(chat_id, msg_id, display).await;
                return Ok(());
            }
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Ok(event) = serde_json::from_str::<Value>(&line) {
                            process_event(&event, &mut full_text, &mut tools_used, &mut captured_session_id);
                        }
                    }
                    Ok(None) => break, // EOF
                    Err(_) => break,
                }
            }
        }

        // Periodic Telegram edit
        let now = Instant::now();
        if full_text.len() > last_edit_len + MIN_DELTA_CHARS
            && now.duration_since(last_edit) >= EDIT_INTERVAL
        {
            let display = build_display(&full_text, &tools_used, &session.model, false);
            let _ = state.bot.edit_message_text(chat_id, msg_id, &display).await;
            last_edit = now;
            last_edit_len = full_text.len();
        }
    }

    // Wait for process to finish
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Process wait error: {}", e))?;

    // Save session ID
    if let Some(sid) = captured_session_id {
        state.update_session(chat_id.0, |s| {
            s.session_id = Some(sid.clone());
        });
    }

    // Final response
    if full_text.is_empty() {
        let _ = state
            .bot
            .edit_message_text(chat_id, msg_id, "⚠️ Pas de réponse de Claude.")
            .await;
    } else {
        // Split long messages
        send_final_response(state, chat_id, msg_id, &full_text, &tools_used, &session.model).await;
    }

    if !status.success() {
        eprintln!("[telegram] Claude exited with: {}", status);
    }

    Ok(())
}

fn process_event(
    event: &Value,
    full_text: &mut String,
    tools_used: &mut Vec<String>,
    session_id: &mut Option<String>,
) {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "assistant" => {
            // Extract text from message.content[] blocks
            if let Some(contents) = event
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                for block in contents {
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                if !text.is_empty() {
                                    full_text.push_str(text);
                                }
                            }
                        }
                        "tool_use" => {
                            if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                                let name = name.to_string();
                                if !tools_used.contains(&name) {
                                    tools_used.push(name);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(sid) = event.get("session_id").and_then(|v| v.as_str()) {
                *session_id = Some(sid.to_string());
            }
        }
        "content_block_start" => {
            if let Some(block) = event.get("content_block") {
                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                        let name = name.to_string();
                        if !tools_used.contains(&name) {
                            tools_used.push(name);
                        }
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Some(delta) = event.get("delta") {
                if delta.get("type").and_then(|v| v.as_str()) == Some("text_delta") {
                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                        full_text.push_str(text);
                    }
                }
            }
        }
        "result" => {
            if let Some(sid) = event.get("session_id").and_then(|v| v.as_str()) {
                *session_id = Some(sid.to_string());
            }
            // Fallback: use result text if nothing captured yet
            if full_text.is_empty() {
                if let Some(result_text) = event.get("result").and_then(|v| v.as_str()) {
                    full_text.push_str(result_text);
                }
            }
        }
        _ => {}
    }
}

/// Send final response, splitting into multiple messages if needed
async fn send_final_response(
    state: &AppState,
    chat_id: ChatId,
    first_msg_id: teloxide::types::MessageId,
    text: &str,
    tools: &[String],
    model: &str,
) {
    let footer = build_footer(tools, model);

    if text.len() <= MAX_MSG_LEN - footer.len() {
        // Fits in one message
        let display = format!("{}{}", text, footer);
        let _ = state.bot.edit_message_text(chat_id, first_msg_id, display).await;
    } else {
        // Split into chunks
        let chunks = split_text(text, MAX_MSG_LEN - 20);

        // First chunk: edit existing message
        let _ = state
            .bot
            .edit_message_text(chat_id, first_msg_id, format!("{}  ➡️", &chunks[0]))
            .await;

        // Middle chunks: new messages
        for chunk in &chunks[1..chunks.len() - 1] {
            let _ = state.bot.send_message(chat_id, format!("{}  ➡️", chunk)).await;
        }

        // Last chunk with footer
        if let Some(last) = chunks.last() {
            let _ = state
                .bot
                .send_message(chat_id, format!("{}{}", last, footer))
                .await;
        }
    }
}

fn build_footer(tools: &[String], model: &str) -> String {
    let mut footer = String::from("\n\n");
    if !tools.is_empty() {
        footer.push_str("🔧 ");
        footer.push_str(&tools.join(", "));
        footer.push('\n');
    }
    footer.push_str(&format!("✅ [{}]", model));
    footer
}

fn build_display(text: &str, tools: &[String], model: &str, finished: bool) -> String {
    let truncated = truncate_text(text);
    let mut display = truncated.to_string();

    if !tools.is_empty() {
        display.push_str("\n\n🔧 ");
        display.push_str(&tools.join(", "));
    }

    if finished {
        display.push_str(&format!("\n\n✅ [{}]", model));
    } else {
        display.push_str("\n\n⏳...");
    }

    display
}

fn truncate_text(text: &str) -> &str {
    if text.len() > MAX_MSG_LEN {
        let mut end = MAX_MSG_LEN;
        while !text.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &text[..end]
    } else {
        text
    }
}

fn split_text(text: &str, max_chunk: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_chunk {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to split at a newline
        let search_end = max_chunk.min(remaining.len());
        let split_at = remaining[..search_end]
            .rfind('\n')
            .unwrap_or_else(|| {
                // Fall back to space
                remaining[..search_end]
                    .rfind(' ')
                    .unwrap_or(search_end)
            });

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start_matches('\n');
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}
