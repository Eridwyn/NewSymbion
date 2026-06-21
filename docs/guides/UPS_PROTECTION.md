# Protection Onduleur (UPS) — Alertes + Extinction propre

> **Safety-critical.** Ce document décrit comment le serveur `symbion` est protégé
> contre une coupure de courant prolongée : alertes Symbion + extinction propre
> automatique au niveau OS. Lis-le entièrement avant de modifier seuils ou services.

Dernière mise à jour : 21 juin 2026.

---

## 1. Topologie et le risque

```
            ┌────────────┐   USB    ┌──────────────────┐
   Secteur ─┤  Onduleur  ├──────────┤  NAS Synology    │  ← maître NUT (upsd :3493)
            │ Eaton 3S550│          │  192.168.1.3     │
            └─────┬──────┘          └──────────────────┘
                  │ prises                    ▲ réseau (LIST VAR / upsc)
                  ▼                           │
            ┌────────────┐                    │
            │  Serveur   │────────────────────┘   ← lecteur NUT réseau
            │  symbion   │   (plugin + watchdog)
            └────────────┘
```

- **NAS = maître NUT** : l'onduleur est branché en USB au NAS, qui expose `upsd` sur `:3493`.
- **Serveur = lecteur réseau** : il lit l'état UPS via le réseau (plugin Symbion + `upsc`).

### Le mode de défaillance redouté (réel)

Lors d'une coupure prolongée :
1. La batterie baisse → le NAS, **à son propre seuil**, entre en safe mode et s'éteint.
2. **L'onduleur continue de débiter sur batterie** — il ne coupe pas ses prises quand
   le NAS s'éteint. Le serveur reste alimenté.
3. `upsd` du NAS est mort → le serveur **n'a plus de source de données UPS**, il tourne en aveugle.
4. La batterie se vide → **coupure brutale du serveur** (« dans les choux »).

### Règle d'or

> **L'extinction propre ne doit JAMAIS dépendre de la stack Symbion**
> (kernel + MQTT + plugin + moteur d'automations + réseau NAS, tous vivants au pire moment).
>
> → **Symbion gère les ALERTES** (best-effort). **NUT/OS gère l'EXTINCTION** (filet de sécurité).

---

## 2. Architecture retenue (Option A : `upsmon`-like + seuil précoce)

Deux couches **indépendantes** :

| Couche | Rôle | Dépend de Symbion ? |
|--------|------|---------------------|
| **Watchdog OS** (`symbion-ups-watchdog`) | Extinction propre du serveur | ❌ Non (lit `upsc` directement) |
| **Automations Symbion** | Alertes Telegram / Email / PWA | ✅ Oui (best-effort) |

L'extinction est déclenchée **tôt** (seuil conservateur), pendant que le NAS est encore
vivant et fournit les données → contourne la course « le NAS s'éteint en premier ».
Un **failsafe** couvre le cas où le NAS meurt malgré tout.

> Alternative non retenue (Option B) : déplacer l'USB de l'onduleur vers le serveur
> (serveur = maître NUT, NAS = esclave) → ordre d'extinction natif correct via HOSTSYNC.
> Plus robuste mais nécessite manip physique + reconfig Synology. À reconsidérer si besoin.

---

## 3. Couche 1 — Watchdog d'extinction (OS-level)

### Fichiers
- Script : `/opt/symbion/bin/ups-shutdown-watchdog.sh` (source : `systemd/ups-shutdown-watchdog.sh`)
- Service : `/etc/systemd/system/symbion-ups-watchdog.service` (source : `systemd/symbion-ups-watchdog.service`)
- Tourne en `root` (requis pour `systemctl poweroff`).

### Logique de déclenchement
Extinction si **sur batterie** (`ups.status` contient `OB`) **ET** l'une des conditions :
- charge ≤ `CHARGE_MIN` (défaut **50 %**), OU
- autonomie ≤ `RUNTIME_MIN` (défaut **600 s** = 10 min),

confirmée sur **`CONFIRM` lectures consécutives** (défaut 2, anti-faux-positif), poll toutes les `POLL` s (défaut 15).

### Failsafe « NAS éteint en premier »
Si `upsc` échoue `FAIL_MAX` fois consécutives (défaut 4 ≈ 60 s) **alors que le dernier
état connu était « sur batterie »** → extinction. Couvre le cas où le NAS coupe avant
le seuil du serveur. (Si le dernier état était « secteur », un échec `upsc` = NAS qui
reboot normalement → **aucune action**.)

### Réglages (env du service)
| Variable | Défaut | Rôle |
|----------|--------|------|
| `UPS_NAME` | `ups@192.168.1.3` | Cible `upsc` |
| `CHARGE_MIN` | `50` | Seuil charge % |
| `RUNTIME_MIN` | `600` | Seuil autonomie s |
| `POLL` | `15` | Intervalle s |
| `CONFIRM` | `2` | Lectures sous seuil avant action |
| `FAIL_MAX` | `4` | Échecs `upsc` (sur batterie) avant failsafe |
| `DRY_RUN` | `1` | **1 = log seulement, n'éteint pas.** 0 = armé. |

### État d'armement
- **`DRY_RUN=1`** : le watchdog logge `[DRY-RUN] poweroff INHIBÉ` au lieu d'éteindre. Sûr, mais **n'offre aucune protection réelle**.
- **`DRY_RUN=0`** : armé, éteint réellement le serveur.

> **État actuel (21 juin 2026) : ARMÉ (`DRY_RUN=0`).** Le watchdog éteindra réellement
> le serveur si l'onduleur passe sur batterie et atteint le seuil.

#### Tester en réel (recommandé AVANT d'armer)
Débrancher l'entrée secteur de l'onduleur 1-2 min et observer :
```bash
sudo journalctl -u symbion-ups-watchdog -f
```
Attendu : `🔋 PASSAGE SUR BATTERIE …` puis le décompte de seuil — **sans extinction** (dry-run).

#### Armer
```bash
sudo sed -i 's/Environment=DRY_RUN=1/Environment=DRY_RUN=0/' /etc/systemd/system/symbion-ups-watchdog.service
sudo systemctl daemon-reload && sudo systemctl restart symbion-ups-watchdog
```
#### Désarmer
```bash
sudo sed -i 's/Environment=DRY_RUN=0/Environment=DRY_RUN=1/' /etc/systemd/system/symbion-ups-watchdog.service
sudo systemctl daemon-reload && sudo systemctl restart symbion-ups-watchdog
```

---

## 4. Couche 2 — Alertes Symbion

Le plugin `symbion-plugin-synology` publie les features UPS sur `symbion/features/update`
(ingérées par le kernel). Les alertes sont des **automations** déclenchées par un trigger
`scheduled` (60s) + une **condition feature** — le moteur n'a pas de trigger « feature
changed », c'est la méthode correcte ici. La **priorité** pilote les canaux :
`P0` = Email + Telegram + PWA ; `P1`/`P2` = Telegram + PWA.

| ID | Nom | Condition | Priorité → canaux | Cooldown |
|----|-----|-----------|-------------------|----------|
| `auto_dbee5101` | [UPS] Coupure secteur | `on_battery == true` | P1 → Telegram, PWA | 30 min |
| `auto_e4a4d158` | [UPS] Batterie critique | `on_battery == true` ET `battery_charge < 50` | P0 → Email, Telegram, PWA | 5 min |

### Features disponibles (pour créer d'autres alertes)
`synology.ups.on_battery` (bool) · `synology.ups.battery_low` (bool) ·
`synology.ups.battery_charge` (nb) · `synology.ups.runtime_seconds` (nb) ·
`synology.ups.load` (nb) · `synology.ups.status` (str)

### Recréer / tester les automations (API)
```bash
KEY=$(sudo grep -oP 'SYMBION_API_KEY=\K.*' /etc/systemd/system/symbion-kernel.service | tr -d '"' | tr -d ' ')
# Lister
curl -sk https://localhost:8443/v1/automations -H "x-api-key: $KEY"
# Test d'exécution (envoie une vraie notif) :
curl -sk -X POST https://localhost:8443/v1/automations/auto_dbee5101/run -H "x-api-key: $KEY"
```
La source de vérité persistée est `data/automations.json` (écrite par le kernel ; ne pas
éditer à la main pendant que le kernel tourne — passer par l'API).

---

## 5. À vérifier côté Synology (important)

> **✅ Confirmé (21 juin 2026) : le NAS est réglé pour s'éteindre sur batterie basse**
> (seuil bas onduleur ≈ 20%), PAS sur un timer court. L'ordre est donc correct :
> **serveur s'éteint à 50% → NAS à ~20%**. Le serveur part en premier pendant que le NAS
> est encore vivant et fournit les données NUT. Le failsafe n'est qu'un filet secondaire.

Panneau de configuration → Matériel et alimentation → UPS :
- « serveur UPS réseau » actif (port 3493 ouvert).
- Entrée en safe mode sur le **seuil bas de l'onduleur** (`battery.charge.low` = 20%) — ✅ en place.

---

## 6. Dépannage

| Symptôme | Vérification |
|----------|--------------|
| Watchdog ne lit rien | `upsc ups@192.168.1.3` depuis le serveur ; NAS joignable ? `nut-client` installé ? |
| Pas d'alerte sur coupure | Plugin actif ? `systemctl status symbion-plugin-synology` ; features dans le kernel : `journalctl -u symbion-kernel | grep synology.ups` |
| Alerte mais pas de Telegram | `systemctl status symbion-plugin-telegram` ; notif publiée : `mosquitto_sub -t 'symbion/notifications/sent@v1'` |
| Pas d'email sur P0 | Config SMTP du kernel (voir `docs/guides/EMAIL_SETUP.md`) |
| Watchdog n'éteint pas | `DRY_RUN` encore à 1 ? (cf. §3 Armer) |

### Services liés
```bash
systemctl status symbion-ups-watchdog      # watchdog OS (extinction)
systemctl status symbion-plugin-synology   # plugin (données + features + alertes)
systemctl status symbion-kernel            # moteur d'automations + notifications
```

---

## 7. Limites connues / dette

- **Fenêtre failsafe ~60 s** : le serveur tourne en aveugle jusqu'à 60 s après la mort du
  NAS avant que le failsafe n'agisse. Acceptable (l'onduleur ne se vide pas en 60 s depuis
  un point de safe-mode NAS), mais réductible via `FAIL_MAX`.
- **Alertes = best-effort** : dépendent de Symbion + MQTT + plugin. Ce n'est PAS la couche
  de sécurité — c'est le watchdog OS qui protège.
- **Option B (serveur = maître NUT)** non implémentée : éliminerait la course par
  conception. À considérer si le réglage Synology ne peut pas être maîtrisé.
