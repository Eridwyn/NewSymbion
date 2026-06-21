#!/usr/bin/env bash
#
# Symbion UPS shutdown watchdog — filet de sécurité OS-level, INDÉPENDANT de Symbion.
#
# Déclenche une extinction propre du serveur quand l'onduleur est sur batterie
# et que le seuil conservateur est atteint, PENDANT que le NAS (source NUT) est
# encore vivant — ce qui contourne la course « le NAS s'éteint en premier ».
#
# Ne dépend PAS du kernel/MQTT/plugin Symbion : lit directement le NUT du NAS via upsc.
#
# Réglages via l'environnement (voir le service systemd) :
#   UPS_NAME      cible upsc (def: ups@192.168.1.3)
#   CHARGE_MIN    seuil charge % (def: 50)
#   RUNTIME_MIN   seuil autonomie s (def: 600 = 10 min)
#   POLL          intervalle s (def: 15)
#   CONFIRM       lectures consécutives sous seuil avant d'agir (def: 2)
#   FAIL_MAX      échecs upsc consécutifs (sur batterie) avant failsafe (def: 4 ~60s)
#   DRY_RUN       1 = log seulement, n'éteint pas (def: 1 — SÛR par défaut)
#   TEST_FORCE_TRIGGER  1 = force la condition (validation DRY-RUN uniquement)

set -uo pipefail

UPS_NAME="${UPS_NAME:-ups@192.168.1.3}"
CHARGE_MIN="${CHARGE_MIN:-50}"
RUNTIME_MIN="${RUNTIME_MIN:-600}"
POLL="${POLL:-15}"
CONFIRM="${CONFIRM:-2}"
FAIL_MAX="${FAIL_MAX:-4}"
DRY_RUN="${DRY_RUN:-1}"
TEST_FORCE_TRIGGER="${TEST_FORCE_TRIGGER:-0}"

log() { echo "$(date '+%Y-%m-%dT%H:%M:%S%z') [ups-watchdog] $*"; }

consecutive_trigger=0
consecutive_fail=0
last_on_battery=0
announced_battery=0

do_shutdown() {
  local reason="$1"
  log "⚠️  DÉCLENCHEMENT extinction propre — $reason"
  if [ "$DRY_RUN" = "1" ]; then
    log "[DRY-RUN] poweroff INHIBÉ (DRY_RUN=1). En prod, le serveur s'éteindrait maintenant."
    consecutive_trigger=0
    return
  fi
  log "Arrêt ordonné du système (systemctl poweroff)…"
  # systemd arrête tous les services dans l'ordre des dépendances.
  /usr/bin/systemctl poweroff
  # Si poweroff échoue/tarde, on n'insiste pas en boucle : on laisse systemd faire.
  sleep 120
}

log "Démarrage — UPS=$UPS_NAME seuils: charge≤${CHARGE_MIN}% OU autonomie≤${RUNTIME_MIN}s | poll=${POLL}s confirm=${CONFIRM} | DRY_RUN=${DRY_RUN}"

while true; do
  out="$(upsc "$UPS_NAME" 2>/dev/null)"
  rc=$?

  if [ $rc -ne 0 ] || [ -z "$out" ]; then
    consecutive_fail=$((consecutive_fail + 1))
    if [ "$last_on_battery" = "1" ]; then
      log "NUT injoignable ($consecutive_fail/$FAIL_MAX) alors qu'on était SUR BATTERIE — le NAS a peut-être coupé."
      if [ "$consecutive_fail" -ge "$FAIL_MAX" ]; then
        do_shutdown "failsafe: NUT injoignable ${FAIL_MAX}x consécutives en mode batterie"
      fi
    else
      log "NUT injoignable ($consecutive_fail) — pas sur batterie au dernier point connu, on attend (pas d'action)."
    fi
    sleep "$POLL"
    continue
  fi
  consecutive_fail=0

  status="$(awk -F': ' '/^ups.status:/{print $2}' <<<"$out")"
  charge="$(awk -F': ' '/^battery.charge:/{print $2}' <<<"$out" | tr -dc '0-9')"
  runtime="$(awk -F': ' '/^battery.runtime:/{print $2}' <<<"$out" | tr -dc '0-9')"
  [ -z "$charge" ] && charge=-1
  [ -z "$runtime" ] && runtime=-1

  on_batt=0
  [[ "$status" == *OB* ]] && on_batt=1
  [ "$TEST_FORCE_TRIGGER" = "1" ] && on_batt=1

  # Annonce des transitions secteur <-> batterie (visibilité dans le journal)
  if [ "$on_batt" = "1" ] && [ "$announced_battery" = "0" ]; then
    log "🔋 PASSAGE SUR BATTERIE — status=$status charge=${charge}% autonomie=${runtime}s"
    announced_battery=1
  elif [ "$on_batt" = "0" ] && [ "$announced_battery" = "1" ]; then
    log "🔌 RETOUR SECTEUR — status=$status charge=${charge}%"
    announced_battery=0
  fi
  last_on_battery=$on_batt

  if [ "$on_batt" = "1" ]; then
    below=0
    { [ "$charge" -ge 0 ] && [ "$charge" -le "$CHARGE_MIN" ]; } && below=1
    { [ "$runtime" -ge 0 ] && [ "$runtime" -le "$RUNTIME_MIN" ]; } && below=1
    [ "$TEST_FORCE_TRIGGER" = "1" ] && below=1

    if [ "$below" = "1" ]; then
      consecutive_trigger=$((consecutive_trigger + 1))
      log "Seuil atteint ($consecutive_trigger/$CONFIRM) — charge=${charge}% autonomie=${runtime}s status=$status"
      if [ "$consecutive_trigger" -ge "$CONFIRM" ]; then
        do_shutdown "sur batterie, charge=${charge}% autonomie=${runtime}s (seuil ${CHARGE_MIN}%/${RUNTIME_MIN}s)"
      fi
    else
      consecutive_trigger=0
    fi
  else
    consecutive_trigger=0
  fi

  sleep "$POLL"
done
