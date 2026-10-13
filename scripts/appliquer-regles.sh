#!/usr/bin/env bash
# Fusionne un fichier de règles dans data/automations.json, kernel ARRÊTÉ.
#
# Le magasin d'automations est chargé en mémoire au démarrage et réécrit par le
# kernel : l'éditer à chaud fait perdre l'ajout au premier enregistrement. D'où
# l'arrêt, la fusion, le redémarrage.
#
#   sudo scripts/appliquer-regles.sh docs/plugins/regles-robot-lumieres.json
set -euo pipefail

AJOUTS="${1:?usage: appliquer-regles.sh <fichier-de-regles.json>}"
DEPOT="$(cd "$(dirname "$0")/.." && pwd)"
MAGASIN="$DEPOT/data/automations.json"

[ -f "$AJOUTS" ] || { echo "fichier introuvable : $AJOUTS" >&2; exit 1; }
python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$AJOUTS"

systemctl stop symbion-kernel
cp -a "$MAGASIN" "$MAGASIN.avant-$(date +%Y%m%d-%H%M%S)"

python3 - "$MAGASIN" "$AJOUTS" <<'PY'
import json, sys
magasin, ajouts = sys.argv[1], sys.argv[2]
d = json.load(open(magasin, encoding="utf-8"))
a = json.load(open(ajouts, encoding="utf-8"))
for cle, regle in a.items():
    etat = "remplacée" if cle in d else "ajoutée"
    d[cle] = regle
    print(f"  {etat} : {regle.get('name', cle)}")
with open(magasin, "w", encoding="utf-8") as f:
    json.dump(d, f, ensure_ascii=False, indent=2)
    f.write("\n")
PY

chown eridwyn:eridwyn "$MAGASIN"
systemctl start symbion-kernel
echo "kernel redémarré — vérifier : journalctl -u symbion-kernel -n 20 | grep -i automation"
