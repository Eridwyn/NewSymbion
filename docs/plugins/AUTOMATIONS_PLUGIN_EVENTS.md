# Les plugins et le moteur d'automations

> Écrit le 13 octobre 2026, en construisant `symbion-plugin-lights` et
> `symbion-plugin-robot`. Ce document dit ce qui marche, et ce qui ne marchait
> pas avant ce jour-là.

## Ce qui existait déjà (et qui marche)

| Pièce | Rôle |
|---|---|
| `ActionDefinition::PluginCommand` | POST HTTP sur une route du plugin via sa socket Unix. **Vraie implémentation**, avec `wrap_protocol` et niveau d'impact. |
| `Condition::Feature` | Lit le `FeatureRegistry` — donc tout ce qu'un plugin publie sur `symbion/features/update`. |
| `PluginAction` à l'enregistrement | Le rule builder de la PWA en fait un **formulaire** au lieu d'un champ JSON libre. |
| Déclencheurs `ModeChange`, `Scheduled`, `PluginHealth`, `Manual` | Fonctionnels. |

## Ce qui manquait : un plugin ne pouvait pas RÉVEILLER une automation

`Trigger::Custom`, `Condition::Custom` et `Action::Custom` existaient dans les
types mais pas dans le moteur :

```rust
// listener.rs, avant
(Trigger::Custom { plugin_name: _, trigger_type: _, .. }, _) => {
    // Custom triggers will be implemented with plugin support
    false
}
```

Et `AutomationEvent` n'avait aucune variante venant d'un plugin. Conséquence
concrète : « allume la lumière de la pièce que le robot nettoie » n'était
faisable qu'en `Scheduled` — soit **60 secondes de retard au mieux** — ou en
faisant appeler `/v1/automations/{id}/run` en dur par le plugin, ce qui
**saute l'évaluation des conditions** (`run_automation` exécute les actions
sans les évaluer).

`set_feature()` ne dispatche rien non plus : publier une feature renseigne les
*conditions*, jamais les *déclencheurs*.

## Ce qui a été ajouté

### `POST /v1/automations/events`

Hors JWT et hors CSRF — un plugin n'a ni session ni nonce — mais gardé deux fois :

1. **un jeton** lu dans `data/plugin-event-token`, écrit par le kernel en
   `0600`. C'est la permission du système de fichiers qui authentifie : le
   kernel écoute sur `0.0.0.0:8443`, un endpoint qui déclenche des actions ne
   peut pas être ouvert. La comparaison du jeton est à temps constant ;
2. **l'enregistrement effectif du plugin** : un nom inventé ne peut pas
   déclencher une règle écrite pour un autre plugin.

```bash
curl -sk https://127.0.0.1:8443/v1/automations/events \
  -H "X-Plugin-Token: $(cat data/plugin-event-token)" \
  -H 'Content-Type: application/json' \
  -d '{"plugin_name":"robot","trigger_type":"zone_changed","data":{"room":"salon"}}'
```

### `Trigger::Custom` : un filtre par inclusion

Chaque clé du `config` du déclencheur doit se retrouver dans les `data` de
l'événement, avec la même valeur. Trois commodités, parce qu'une règle écrite
à la main ne doit pas échouer en silence :

- **`config` vide** → tout événement du bon plugin et du bon type passe ;
- **valeur en tableau** → « l'une de ces valeurs » (`{"room": ["salon","cuisine"]}`) ;
- **clé pointée** → descend dans les données imbriquées (`{"zone.id": 10003}`) ;
- **égalité tolérante** → `10000` vaut `"10000"`, `"Cuisine"` vaut `"cuisine"`.

## Les événements publiés par les deux plugins

### `robot` (Deebot T50 Pro Omni, Matter local)

| `trigger_type` | `data` |
|---|---|
| `zone_changed` | `zone_id`, `zone` (nom Ecovacs), `room` (pièce principale), `rooms` (**toutes** les pièces couvertes), `previous_zone_id`, `cleaning` |
| `run_mode_changed` | `mode`, `previous_mode` |
| `cleaning_started` | `clean_mode`, `zones` |
| `cleaning_finished` | `battery`, `last_zone` |
| `phase_changed` | `phase`, `previous_phase` |
| `drying_started` / `drying_finished` | — |
| `error_changed` | `code`, `error` (bac plein, réservoir vide…) |
| `battery_low` | `battery`, `threshold` |

### `lights` (ampoules Tuya en local)

| `trigger_type` | `data` |
|---|---|
| `light_changed` | `light_id`, `room`, `on`, `online`, `brightness`, `color_temp_k`, `was_on` |
| `room_any_on_changed` | `room`, `any_on` |

Les deux plugins ne poussent que ce qui a **changé**. Une automation qui se
redéclencherait à chaque poussée du matériel rallumerait la pièce que le
porteur vient d'éteindre.

## La règle visée : allumer la pièce qu'il nettoie

Deux écarts entre la carte du robot et les lampes, et aucun n'est un détail :

1. **les noms diffèrent** — « Salle de séjour » chez Ecovacs, « salon » pour
   les lampes ;
2. **une zone couvre plusieurs pièces** — la cuisine et le salon sont tous
   deux dans « Salle de séjour » au sens de la carte.

D'où une table `[rooms]` qui accepte une LISTE par zone, et un événement qui
porte `rooms` (toutes les pièces couvertes) en plus de `zone` et `room`.
Une règle se vise sur **`rooms`**, pas sur `room`.

### Déclencheur large, conditions sur l'état vivant

Le filtrage ne va pas dans le `config` du déclencheur mais dans les
**conditions** : le déclencheur dit « le robot a changé de zone », et deux
conditions vérifient l'état **au moment d'agir** —

- `robot.cleaning` **equals** `true` — il nettoie vraiment, il ne rentre pas
  à la base ni ne traverse ;
- `robot.rooms` **contains** `"salon"` — il est bien dans une zone qui couvre
  cette pièce.

C'est plus juste qu'un filtre sur la charge de l'événement : un événement peut
avoir une seconde de retard, les features sont relues à l'instant de
l'évaluation. `robot.rooms` est une **`StringList`** côté kernel, donc
`contains` compare élément par élément : « salon » ne matche pas dans
« salon-bis ».

> Ordre garanti côté plugin : les features partent **avant** l'événement et le
> plugin attend l'acquittement du courtier (`attendre_remise()`). Sans ça la
> condition pourrait être évaluée sur la zone précédente — et allumer la pièce
> qu'il vient de quitter.

```json
{
  "name": "🧹→💡 Salon : allumer quand le robot y passe",
  "triggers": {
    "operator": "and",
    "triggers": [{
      "type": "custom",
      "plugin_name": "robot",
      "trigger_type": "zone_changed",
      "config": {}
    }]
  },
  "conditions": {
    "operator": "and",
    "conditions": [
      { "type": "feature", "feature_id": "robot.cleaning", "operator": "equals",   "value": true },
      { "type": "feature", "feature_id": "robot.rooms",    "operator": "contains", "value": "salon" }
    ]
  },
  "actions": [{
    "type": "plugin_command",
    "plugin": "lights",
    "route": "actions/room_on",
    "payload": { "room": "salon" },
    "impact_level": "Low"
  }],
  "cooldown_seconds": 10
}
```

Les deux règles réellement utiles chez moi sont prêtes dans
`docs/plugins/regles-robot-lumieres.json`, à fusionner **kernel arrêté** (le
magasin est chargé en mémoire et réécrit par le kernel : l'éditer à chaud fait
perdre l'ajout au premier enregistrement) :

```bash
sudo scripts/appliquer-regles.sh docs/plugins/regles-robot-lumieres.json
```

> **Le magasin de règles est SQLite**, table `automations` de
> `data/symbion.db`. `data/automations.json` n'est qu'un **repli** : le kernel
> ne le lit que si la table est vide, et le réécrit ensuite depuis sa mémoire.
> Y écrire ne sert à rien — piège rencontré au premier essai, le kernel
> annonçant tranquillement « Loaded 19 automation rules from SQLite » alors que
> le JSON en contenait 24. Les horodatages en base sont du **RFC3339**
> (`2026-10-13T16:30:00Z`), pas le format étendu du JSON.

**Trois** règles : **salon**, **cuisine** (toutes deux dans la zone « Salle de
séjour ») et **chambre**. Le Couloir et la Salle de bains ont une zone mais pas
de lampe pilotable.

Limite à connaître : `payload` est **statique**. Le moteur ne substitue pas
encore les données de l'événement dans l'action, donc une règle = une pièce.
Pour les quatre pièces cartographiées, c'est quatre règles, chacune avec
`config: {"room": "salon", "cleaning": true}` et le `payload` correspondant.
Un gabarit de payload (`{{event.room}}`) serait le prochain pas utile.

## Un piège trouvé au déploiement : l'en-tête `Host`

Le moniteur de santé du kernel envoyait `GET /health` **sans en-tête `Host`**.
HTTP/1.1 l'exige ; les plugins écrits en hyper (tous les plugins Rust) le
laissaient passer, mais aiohttp répond `400 Missing 'Host' header`. Résultat :
les deux plugins Python étaient déclarés en panne (`Failed to parse health
response`) alors qu'ils répondaient parfaitement sur leur socket — et un
événement `plugin_health` partait à chaque cycle. Corrigé dans
`plugin_health.rs` et `plugin_proxy.rs::public_library_proxy`.

À retenir pour tout futur plugin non-Rust : un serveur HTTP strict refusera
les requêtes internes du kernel qui n'ont pas de `Host`.

## Les règles d'extinction

Le symétrique a sa propre difficulté : **il n'existe aucun déclencheur sur
changement de feature**. La présence arrive par MQTT et renseigne le registre,
elle ne réveille aucune automation. D'où deux règles complémentaires :

| Règle | Déclencheur | Conditions |
|---|---|---|
| 🌙 Personne à la maison | `Scheduled` 60 s | `presence.anyone_home` = false · `lights.any_on` = true · `robot.cleaning` = false |
| 🧹 Ménage fini | `custom robot/cleaning_finished` | `presence.anyone_home` = false |

Trois choses à noter :

- **`lights.any_on`** évite que la règle tourne à vide : sans elle, « tout
  éteindre » s'exécuterait toutes les minutes pour rien et saturerait
  l'historique. Une lampe injoignable allumée n'y compte pas — on ne peut rien
  éteindre qu'on ne joint pas ;
- **`robot.cleaning` = false** empêche les deux familles de règles de se
  battre. Sans elle, pendant le tour du robot en l'absence du porteur, chaque
  pièce allumée serait éteinte une minute plus tard : les lampes
  clignoteraient pendant tout son tour ;
- la seconde règle n'est là que pour la **réactivité** : à la fin du ménage on
  a un vrai événement, autant éteindre tout de suite plutôt qu'attendre le
  sondage de la minute.

Le sondage à 60 s est cohérent avec la TTL de la présence (90 s) : on ne lit
jamais une présence périmée.

### La panne du 15 octobre : « personne à la maison » ne connaissait qu'un seul téléphone

Les deux règles d'extinction ont été **désactivées** le jour de leur mise en
service. `presence.anyone_home` est calculé par le plugin freebox sur la seule
liste `[devices.*]` de sa configuration — et un seul téléphone y était déclaré.
Dès que le porteur sortait, « personne à la maison » devenait vrai alors que sa
compagne était là, et les lumières s'éteignaient **toutes les 5 minutes**, soit
exactement la durée du cooldown.

Trois corrections :

1. **les appareils suivis se déclarent depuis la PWA** (widget Freebox →
   ⚙️ Gérer). Le plugin expose `GET /presence`, `GET /devices` (ce que la
   Freebox voit, actifs d'abord), `GET|POST /tracked`, `DELETE /tracked/{id}`.
   La liste vit dans `data/freebox-devices.json`, écrit de façon atomique ; la
   section `[devices.*]` du TOML n'en est plus que la graine ;
2. **un nom inconnu de la Freebox est refusé** à l'ajout, et signalé dans la
   PWA s'il devient inconnu plus tard. Un appareil mal nommé compterait comme
   éternellement absent — exactement l'erreur qui éteint une maison sans rien
   expliquer ;
3. **l'absence doit être SOUTENUE** : le plugin publie
   `presence.away_minutes`, et la règle exige 10 minutes. Un Wi-Fi qui décroche
   trois secondes n'éteint plus rien.

Les deux règles restent désactivées jusqu'à ce que tous les téléphones de la
maison soient déclarés. **Une règle d'extinction est aussi dangereuse que la
liste sur laquelle elle s'appuie est incomplète.**

## Éprouvé de bout en bout

Le 13 octobre 2026, features mises à l'état voulu puis événement poussé :

```
[automations] dispatched custom event robot/zone_changed (1 receivers)
[automations] ✅ TRIGGER MATCH: '🧹→💡 Chambre …'
[automations]   ❌ Conditions NOT met: ["feature 'robot.rooms' list contains 'chambre' → false"]
[automations] ✅ TRIGGER MATCH: '🧹→💡 Cuisine …'
[automations]   ✓ Conditions met, executing 1 action(s)
[automations] ✅ plugin_command(plugin='lights', route='/actions/room_on') APPROVED (trust: 0.83)
[automations] 🔌 plugin_command lights/actions/room_on OK
```

Cuisine et salon allumés (tous deux dans « Salle de séjour »), chambre non
touchée. La `StringList` fait bien son travail : `list contains` et non une
recherche de sous-chaîne.

## Ce qui reste placeholder

`Condition::Custom` et `Action::Custom` ne sont **toujours pas** implémentés —
et ils ne manquent pas : `Condition::Feature` couvre les conditions (les deux
plugins publient leurs features), et `PluginCommand` couvre les actions mieux
que `Action::Custom` ne l'aurait fait, puisqu'il atteint n'importe quelle
route HTTP du plugin.
