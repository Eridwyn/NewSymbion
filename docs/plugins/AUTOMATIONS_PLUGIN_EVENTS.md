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
| `zone_changed` | `zone_id`, `zone` (nom Ecovacs), `room` (pièce Symbion), `previous_zone_id`, `cleaning` |
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

Les pièces du robot et celles des lampes ne portent pas les mêmes noms
(« Salle de séjour » chez Ecovacs, « salon » pour les lampes). La table
`[rooms]` de `robot.toml` fait la traduction, et l'événement porte **les
deux** — `zone` pour le nom Ecovacs, `room` pour la pièce Symbion. Une règle
se vise donc sur `room`.

```json
{
  "name": "Lumière de la pièce nettoyée",
  "triggers": [{
    "type": "custom",
    "plugin_name": "robot",
    "trigger_type": "zone_changed",
    "config": { "cleaning": true }
  }],
  "conditions": { "operator": "and", "conditions": [] },
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

Deux et pas quatre : seules **chambre** et **salon** ont à la fois une zone sur
la carte du robot et une lampe. Le Couloir et la Salle de bains n'ont pas de
lampe pilotable, et la cuisine n'est pas cartographiée.

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

## Ce qui reste placeholder

`Condition::Custom` et `Action::Custom` ne sont **toujours pas** implémentés —
et ils ne manquent pas : `Condition::Feature` couvre les conditions (les deux
plugins publient leurs features), et `PluginCommand` couvre les actions mieux
que `Action::Custom` ne l'aurait fait, puisqu'il atteint n'importe quelle
route HTTP du plugin.
