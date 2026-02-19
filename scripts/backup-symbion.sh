#!/bin/bash
# Symbion — Backup automatique des donnees JSON
# Usage: ./backup-symbion.sh [--quiet]
#
# Cron (quotidien 3h du matin) :
#   0 3 * * * /home/eridwyn/RustroverProjects/NewSymbion/scripts/backup-symbion.sh --quiet
#
# Ou via systemd timer (recommande) :
#   sudo cp systemd/symbion-backup.{service,timer} /etc/systemd/system/
#   sudo systemctl enable --now symbion-backup.timer

set -euo pipefail

export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# --- Configuration ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DATA_DIR="${SYMBION_DATA_DIR:-$PROJECT_DIR/data}"
BACKUP_ROOT="${SYMBION_BACKUP_DIR:-${HOME}/symbion-backups}"
RETENTION_DAYS=30
QUIET="${1:-}"
DATE=$(date '+%Y-%m-%d_%H%M')
BACKUP_DIR="$BACKUP_ROOT/$DATE"

log() {
    [[ "$QUIET" == "--quiet" ]] && return
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

error() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1" >&2
}

# --- Pre-checks ---
if [[ ! -d "$DATA_DIR" ]]; then
    error "Data directory $DATA_DIR does not exist"
    exit 1
fi

# --- Create backup ---
mkdir -p "$BACKUP_DIR"

log "Backup Symbion data → $BACKUP_DIR"

# Copier tous les fichiers JSON
file_count=0
total_size=0
for f in "$DATA_DIR"/*.json; do
    [[ -f "$f" ]] || continue
    cp "$f" "$BACKUP_DIR/"
    size=$(stat --format=%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null || echo 0)
    total_size=$((total_size + size))
    file_count=$((file_count + 1))
done

if [[ $file_count -eq 0 ]]; then
    error "No JSON files found in $DATA_DIR"
    rmdir "$BACKUP_DIR" 2>/dev/null
    exit 1
fi

# --- Compresser ---
archive="$BACKUP_ROOT/symbion-backup-$DATE.tar.gz"
tar -czf "$archive" -C "$BACKUP_ROOT" "$DATE"
rm -rf "$BACKUP_DIR"

archive_size=$(stat --format=%s "$archive" 2>/dev/null || stat -f%z "$archive" 2>/dev/null || echo 0)
archive_size_kb=$((archive_size / 1024))

log "$file_count fichiers sauvegardes ($((total_size / 1024)) Ko raw → ${archive_size_kb} Ko compresse)"

# --- Rotation : supprimer les backups > RETENTION_DAYS jours ---
deleted=0
find "$BACKUP_ROOT" -name "symbion-backup-*.tar.gz" -mtime +$RETENTION_DAYS -type f | while read old; do
    rm -f "$old"
    deleted=$((deleted + 1))
done

if [[ $deleted -gt 0 ]]; then
    log "Rotation: $deleted ancien(s) backup(s) supprime(s) (> ${RETENTION_DAYS}j)"
fi

# --- Resume ---
backup_count=$(find "$BACKUP_ROOT" -name "symbion-backup-*.tar.gz" -type f | wc -l)
disk_usage=$(du -sh "$BACKUP_ROOT" 2>/dev/null | cut -f1)

log "Total: $backup_count backups, $disk_usage sur disque"
log "Backup termine avec succes"
