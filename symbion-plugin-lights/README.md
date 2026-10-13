# symbion-plugin-lights

Ampoules **Tuya / Smart Life pilotées en local** sur le LAN : ni cloud, ni
Alexa, ni Internet. Suit `docs/plugins/LIGHTS_DESIGN.md`, en Python avec
`tinytuya` (option A du §3 — le protocole Tuya chiffré est la partie difficile,
et `tinytuya` la résout).

## Ce qu'il expose

- **API** (socket Unix, proxifiée `/v1/plugin-api/lights/*`) :
  `GET /health` · `GET /lights` · `GET|PUT /lights/{id}` ·
  `PUT /lights/groups/{room}` · `GET|POST /scenes` ·
  `PUT /scenes/{id}/activate` · `POST /actions/{room_on,room_off,all_off,set_scene,set_light}`
- **MQTT** : `symbion/lights/{id}` et `symbion/lights/summary` (retenus),
  features `lights.{room}.any_on` · `lights.{id}.on` · `lights.{id}.brightness`
- **Événements** poussés au kernel : `light_changed`, `room_any_on_changed`
  (voir `docs/plugins/AUTOMATIONS_PLUGIN_EVENTS.md`)

## Installation

```bash
python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
install -m 600 lights.example.toml /opt/symbion/config/lights.toml   # puis remplir
sudo cp ../systemd/symbion-plugin-lights.service /etc/systemd/system/
sudo systemctl enable --now symbion-plugin-lights
```

Les `local_key` sont des **secrets** : la config reste hors du dépôt, en 0600.
Pour les obtenir : `docs/plugins/LIGHTS_DESIGN.md` §13.

## Essais

```bash
./.venv/bin/python essais.py     # sans matériel, sans kernel, sans MQTT
```

## Pièges

- **Une seule connexion locale par ampoule.** Un deuxième contrôleur local
  (LocalTuya de Home Assistant, par exemple) sur la même ampoule les fait se
  déconnecter mutuellement. Le canal cloud, lui, ne compte pas : Alexa
  continue de marcher, et Symbion **voit** ce qu'Alexa a changé.
- **Réserver les IP** des ampoules en DHCP statique. Une IP qui change casse
  le contrôle local sans autre symptôme qu'une lampe « offline ».
- **Ré-appairer une ampoule dans l'app Smart Life change son `local_key`** :
  il faut la ré-extraire.
- La luminosité 0 n'existe pas chez Tuya : une lampe à 0 % est une lampe
  éteinte, et c'est `on` qui le dit.
