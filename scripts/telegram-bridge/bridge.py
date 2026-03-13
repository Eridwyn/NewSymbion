#!/usr/bin/env python3
"""
Symbion Telegram-Claude Code Bridge v3

- Streaming: reponse progressive en direct dans Telegram
- Sessions persistantes
- Modele haiku par defaut (rapide), switch facile
- Effort low pour rapidite
"""

import asyncio
import json
import logging
import os
import signal
import time
from pathlib import Path

from telegram import Update
from telegram.constants import ParseMode, ChatAction
from telegram.ext import (
    Application,
    CommandHandler,
    MessageHandler,
    ContextTypes,
    filters,
)


def load_config():
    env_path = Path(__file__).parent / "config.env"
    config = {}
    if env_path.exists():
        for line in env_path.read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, value = line.split("=", 1)
                config[key.strip()] = value.strip()
    for key in ["TELEGRAM_BOT_TOKEN", "ALLOWED_USER_IDS", "CLAUDE_PATH",
                "CLAUDE_TIMEOUT", "CLAUDE_WORKDIR"]:
        if key in os.environ:
            config[key] = os.environ[key]
    return config


CONFIG = load_config()
BOT_TOKEN = CONFIG["TELEGRAM_BOT_TOKEN"]
ALLOWED_IDS = {int(uid) for uid in CONFIG.get("ALLOWED_USER_IDS", "").split(",")}
CLAUDE_PATH = CONFIG.get("CLAUDE_PATH", "claude")
CLAUDE_TIMEOUT = int(CONFIG.get("CLAUDE_TIMEOUT", "600"))
CLAUDE_WORKDIR = CONFIG.get("CLAUDE_WORKDIR", str(Path.home()))

# Streaming config
STREAM_UPDATE_INTERVAL = 1.5  # seconds between Telegram message edits
STREAM_MIN_DELTA = 30  # minimum new chars before updating message

logging.basicConfig(
    format="%(asctime)s [%(levelname)s] %(message)s",
    level=logging.INFO,
)
log = logging.getLogger("bridge")

# Per-user state
active_processes: dict[int, asyncio.subprocess.Process] = {}
user_sessions: dict[int, str | None] = {}
user_models: dict[int, str] = {}


def is_allowed(user_id: int) -> bool:
    return user_id in ALLOWED_IDS


def split_message(text: str, max_len: int = 4096) -> list[str]:
    if len(text) <= max_len:
        return [text]
    chunks = []
    while text:
        if len(text) <= max_len:
            chunks.append(text)
            break
        cut = text.rfind("\n", 0, max_len)
        if cut == -1 or cut < max_len // 2:
            cut = max_len
        chunks.append(text[:cut])
        text = text[cut:].lstrip("\n")
    return chunks


async def safe_edit(msg, text: str):
    """Edit message, try markdown then plain text. Truncate if needed."""
    text = text[:4096]
    try:
        await msg.edit_text(text, parse_mode=ParseMode.MARKDOWN)
    except Exception:
        try:
            await msg.edit_text(text)
        except Exception:
            pass


async def safe_send(message, text: str):
    """Send message, try markdown then plain."""
    try:
        return await message.reply_text(text, parse_mode=ParseMode.MARKDOWN)
    except Exception:
        return await message.reply_text(text)


# ─── Commands ───────────────────────────────────────────────────────

async def cmd_start(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        await update.message.reply_text("Acces refuse.")
        return
    await update.message.reply_text(
        "🦀 *Symbion Claude Bridge v3*\n\n"
        "Envoie un message → Claude repond en streaming.\n\n"
        "*Commandes:*\n"
        "/new — Nouvelle conversation\n"
        "/continue — Reprendre la session\n"
        "/cancel — Annuler\n"
        "/status — Etat\n"
        "/model — Changer modele (haiku/sonnet/opus)\n"
        "/effort — Changer effort (low/medium/high)",
        parse_mode=ParseMode.MARKDOWN,
    )


async def cmd_new(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    user_sessions[update.effective_user.id] = None
    await update.message.reply_text("🆕 Nouvelle conversation.")


async def cmd_continue_session(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    uid = update.effective_user.id
    user_sessions[uid] = "__continue__"
    await update.message.reply_text("▶️ Prochaine requete = reprise session.")


async def cmd_cancel(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    uid = update.effective_user.id
    proc = active_processes.get(uid)
    if proc:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
        except ProcessLookupError:
            pass
        active_processes.pop(uid, None)
        await update.message.reply_text("⏹ Annule.")
    else:
        await update.message.reply_text("Rien en cours.")


async def cmd_status(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    uid = update.effective_user.id
    busy = uid in active_processes
    model = user_models.get(uid, "haiku")
    session = user_sessions.get(uid)
    s = f"`{session[:12]}…`" if session and session != "__continue__" else "—"
    await update.message.reply_text(
        f"{'🔴' if busy else '🟢'} *Bridge v3*\n"
        f"Modele: `{model}`\n"
        f"Session: {s}\n"
        f"Workdir: `{CLAUDE_WORKDIR}`",
        parse_mode=ParseMode.MARKDOWN,
    )


async def cmd_model(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    uid = update.effective_user.id
    args = update.message.text.split(maxsplit=1)
    if len(args) < 2:
        current = user_models.get(uid, "haiku")
        await update.message.reply_text(
            f"Modele: *{current}*\n`/model haiku` · `/model sonnet` · `/model opus`",
            parse_mode=ParseMode.MARKDOWN,
        )
        return
    user_models[uid] = args[1].strip()
    await update.message.reply_text(f"Modele → *{args[1].strip()}*", parse_mode=ParseMode.MARKDOWN)


user_efforts: dict[int, str] = {}

async def cmd_effort(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        return
    uid = update.effective_user.id
    args = update.message.text.split(maxsplit=1)
    if len(args) < 2:
        current = user_efforts.get(uid, "low")
        await update.message.reply_text(
            f"Effort: *{current}*\n`/effort low` · `/effort medium` · `/effort high`",
            parse_mode=ParseMode.MARKDOWN,
        )
        return
    user_efforts[uid] = args[1].strip()
    await update.message.reply_text(f"Effort → *{args[1].strip()}*", parse_mode=ParseMode.MARKDOWN)


# ─── Core: streaming Claude ─────────────────────────────────────────

async def run_claude_streaming(prompt: str, user_id: int, status_msg):
    """Run claude with stream-json output and update Telegram message live."""
    effort = user_efforts.get(user_id, "low")
    cmd = [
        CLAUDE_PATH, "-p", prompt,
        "--dangerously-skip-permissions",
        "--disable-slash-commands",
        "--effort", effort,
        "--output-format", "stream-json",
        "--verbose",
        "--append-system-prompt",
        "Reponds en francais, concis et direct.",
    ]

    session = user_sessions.get(user_id)
    if session == "__continue__":
        cmd.append("--continue")
    elif session:
        cmd.extend(["--resume", session])

    model = user_models.get(user_id, "haiku")
    cmd.extend(["--model", model])

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        cwd=CLAUDE_WORKDIR,
        start_new_session=True,
    )
    active_processes[user_id] = proc

    collected_text = ""
    last_update = 0
    last_sent_len = 0
    session_id = None
    tool_active = False
    tool_name = ""

    try:
        while True:
            try:
                line = await asyncio.wait_for(
                    proc.stdout.readline(), timeout=CLAUDE_TIMEOUT
                )
            except asyncio.TimeoutError:
                await safe_edit(status_msg, collected_text or "⏱ Timeout.")
                try:
                    os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
                except ProcessLookupError:
                    pass
                return collected_text

            if not line:
                break

            raw = line.decode("utf-8", errors="replace").strip()
            if not raw:
                continue

            try:
                event = json.loads(raw)
            except json.JSONDecodeError:
                continue

            etype = event.get("type", "")

            # Capture session ID
            if "session_id" in event:
                session_id = event["session_id"]

            # Extract text from assistant messages
            if etype == "assistant" and "message" in event:
                msg_data = event["message"]
                if isinstance(msg_data, dict):
                    for block in msg_data.get("content", []):
                        if isinstance(block, dict):
                            if block.get("type") == "text":
                                collected_text = block.get("text", "")
                            elif block.get("type") == "tool_use":
                                tool_name = block.get("name", "")
                                tool_active = True

            elif etype == "content_block_delta":
                delta = event.get("delta", {})
                if delta.get("type") == "text_delta":
                    collected_text += delta.get("text", "")

            # Final result
            elif etype == "result":
                result_text = event.get("result", "")
                if result_text:
                    collected_text = result_text
                if event.get("session_id"):
                    session_id = event["session_id"]

            # Tool use indicators
            elif etype == "tool_use":
                tool_name = event.get("name", event.get("tool", ""))
                tool_active = True

            elif etype == "tool_result":
                tool_active = False

            # Show tool activity
            if tool_active and tool_name:
                now = time.monotonic()
                if now - last_update > STREAM_UPDATE_INTERVAL:
                    progress = collected_text[:3800] if collected_text else ""
                    indicator = f"{progress}\n\n⚙️ _{tool_name}..._" if progress else f"⚙️ _{tool_name}..._"
                    await safe_edit(status_msg, indicator)
                    last_update = now
                    tool_name = ""

            # Update Telegram message periodically
            now = time.monotonic()
            new_chars = len(collected_text) - last_sent_len
            if (not tool_active
                    and new_chars >= STREAM_MIN_DELTA
                    and now - last_update >= STREAM_UPDATE_INTERVAL
                    and collected_text):
                display = collected_text[:4096]
                await safe_edit(status_msg, display)
                last_update = now
                last_sent_len = len(collected_text)

    except Exception as e:
        log.exception("Streaming error")
        return collected_text or f"Erreur: {e}"
    finally:
        active_processes.pop(user_id, None)
        if session_id:
            user_sessions[user_id] = session_id

        # Wait for process to finish
        try:
            await asyncio.wait_for(proc.wait(), timeout=5)
        except asyncio.TimeoutError:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
            except ProcessLookupError:
                pass

    return collected_text


# ─── Message handler ────────────────────────────────────────────────

async def handle_message(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    if not is_allowed(update.effective_user.id):
        await update.message.reply_text("Acces refuse.")
        return

    uid = update.effective_user.id
    prompt = update.message.text

    if uid in active_processes:
        await update.message.reply_text("⏳ Requete en cours... /cancel pour annuler.")
        return

    await update.message.chat.send_action(ChatAction.TYPING)
    status_msg = await update.message.reply_text("💭 ...")

    try:
        response = await run_claude_streaming(prompt, uid, status_msg)

        if not response:
            await safe_edit(status_msg, "Pas de reponse.")
            return

        # Final update — if response is longer than 4096, split
        if len(response) <= 4096:
            await safe_edit(status_msg, response)
        else:
            # Delete status and send in chunks
            try:
                await status_msg.delete()
            except Exception:
                pass
            chunks = split_message(response)
            for chunk in chunks:
                await safe_send(update.message, chunk)

    except Exception as e:
        log.exception("Error handling message")
        await safe_edit(status_msg, f"Erreur: {e}")


# ─── Main ───────────────────────────────────────────────────────────

def main():
    log.info("Starting Symbion Telegram-Claude Bridge v3")
    log.info(f"Claude: {CLAUDE_PATH}")
    log.info(f"Workdir: {CLAUDE_WORKDIR}")
    log.info(f"Allowed users: {ALLOWED_IDS}")

    app = Application.builder().token(BOT_TOKEN).build()

    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("help", cmd_start))
    app.add_handler(CommandHandler("new", cmd_new))
    app.add_handler(CommandHandler("continue", cmd_continue_session))
    app.add_handler(CommandHandler("cancel", cmd_cancel))
    app.add_handler(CommandHandler("status", cmd_status))
    app.add_handler(CommandHandler("model", cmd_model))
    app.add_handler(CommandHandler("effort", cmd_effort))
    app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_message))

    log.info("Bridge v3 ready!")
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
