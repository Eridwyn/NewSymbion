#!/usr/bin/env bash
# Insère ou remplace des règles d'automation, kernel ARRÊTÉ.
#
# La VÉRITÉ est dans SQLite (`data/symbion.db`, table `automations`).
# `data/automations.json` n'est qu'un REPLI : le kernel ne le lit que si la
# table est vide, et le réécrit depuis sa mémoire. Y écrire ne sert à rien.
#
# Kernel arrêté parce que les règles sont chargées en mémoire au démarrage et
# réécrites ensuite : une insertion à chaud serait écrasée au premier
# enregistrement.
#
#   sudo scripts/appliquer-regles.sh docs/plugins/regles-robot-lumieres.json
set -euo pipefail

AJOUTS="${1:?usage: appliquer-regles.sh <fichier-de-regles.json>}"
DEPOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="$DEPOT/data/symbion.db"

[ -f "$AJOUTS" ] || { echo "fichier introuvable : $AJOUTS" >&2; exit 1; }
[ -f "$BASE" ]   || { echo "base introuvable : $BASE" >&2; exit 1; }
python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$AJOUTS"

systemctl stop symbion-kernel
cp -a "$BASE" "$BASE.avant-$(date +%Y%m%d-%H%M%S)"

python3 - "$BASE" "$AJOUTS" <<'PY'
import json, sqlite3, sys
from datetime import datetime, timezone

base, ajouts = sys.argv[1], sys.argv[2]
regles = json.load(open(ajouts, encoding="utf-8"))
# RFC3339 : c'est ce que le kernel sait relire (OffsetDateTime::parse).
maintenant = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

con = sqlite3.connect(base)
con.execute("PRAGMA foreign_keys = ON")
for cle, r in regles.items():
    ident = r.get("id") or cle
    existe = con.execute("select 1 from automations where id = ?", (ident,)).fetchone()
    con.execute(
        """insert into automations
             (id, name, description, category, goal_mode, enabled,
              triggers_json, conditions_json, actions_json, cooldown_seconds,
              trusted, skip_if_same_mode, auto_created,
              last_executed_at, execution_count, created_at, updated_at, deleted_at)
           values (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
           on conflict(id) do update set
             name = excluded.name,
             description = excluded.description,
             category = excluded.category,
             goal_mode = excluded.goal_mode,
             enabled = excluded.enabled,
             triggers_json = excluded.triggers_json,
             conditions_json = excluded.conditions_json,
             actions_json = excluded.actions_json,
             cooldown_seconds = excluded.cooldown_seconds,
             trusted = excluded.trusted,
             skip_if_same_mode = excluded.skip_if_same_mode,
             updated_at = excluded.updated_at,
             deleted_at = null""",
        (
            ident,
            r["name"],
            r.get("description"),
            r.get("category") or "custom",
            r.get("goal_mode"),
            1 if r.get("enabled", True) else 0,
            json.dumps(r.get("triggers"), ensure_ascii=False) if r.get("triggers") else None,
            json.dumps(r.get("conditions"), ensure_ascii=False) if r.get("conditions") else None,
            json.dumps(r.get("actions") or [], ensure_ascii=False),
            int(r.get("cooldown_seconds", 60)),
            r.get("trusted"),
            r.get("skip_if_same_mode"),
            r.get("auto_created"),
            None,
            0,
            maintenant,
            maintenant,
            None,
        ),
    )
    print(f"  {'remplacée' if existe else 'ajoutée'} : {r['name']}")
con.commit()
vivantes = con.execute("select count(*) from automations where deleted_at is null").fetchone()[0]
con.close()
print(f"  → {vivantes} règles vivantes en base")
PY

chown eridwyn:eridwyn "$BASE"
systemctl start symbion-kernel

# Le dashboard est en BindsTo= sur le kernel : l'arrêter emporte le dashboard,
# et un arrêt par DÉPENDANCE n'enclenche pas son Restart=always. Sans cette
# ligne, chaque application de règles laisse l'interface web morte jusqu'à ce
# que quelqu'un s'en aperçoive — c'est arrivé deux fois le 15/10/2026.
systemctl is-active --quiet symbion-dashboard || systemctl start symbion-dashboard

sleep 4
journalctl -u symbion-kernel --since "-1min" -o cat --no-pager | grep -i "automation rules from" | tail -1
