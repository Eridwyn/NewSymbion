# symbion-plugin-robot

**Deebot T50 Pro Omni Gen3** piloté en **Matter local**, via
`python-matter-server` (conteneur `matter-server`, `ws://127.0.0.1:5580/ws`).
Le robot garde Alexa en parallèle : un appareil Matter accepte plusieurs
fabriques.

## Ce qu'il expose

- **API** (socket Unix, proxifiée `/v1/plugin-api/robot/*`) :
  `GET /health` · `GET /robot` · `GET /robot/zones` ·
  `POST /actions/{clean_rooms,clean_all,stop_and_dock,dock,set_clean_mode,clear_selection,pause,resume}`
- **MQTT** : `symbion/robot/state`, `symbion/robot/zones` (retenus),
  features `robot.{run_mode,phase,zone,room,cleaning,docked,drying,battery,error}`
- **Une zone de la carte peut couvrir plusieurs pièces** au sens des lampes
  (chez moi la cuisine et le salon sont tous deux dans « Salle de séjour »).
  La table `[rooms]` accepte donc une liste, et `robot.rooms` porte toutes les
  pièces couvertes — c'est sur elle qu'une règle se vise.
- **Événements** poussés au kernel : `zone_changed`, `run_mode_changed`,
  `cleaning_started`, `cleaning_finished`, `phase_changed`, `drying_started`,
  `drying_finished`, `error_changed`, `battery_low`

## Installation

```bash
python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
install -m 644 robot.example.toml /opt/symbion/config/robot.toml
sudo cp ../systemd/symbion-plugin-robot.service /etc/systemd/system/
sudo systemctl enable --now symbion-plugin-robot
```

**`/home/eridwyn/matter/data` NE DOIT PAS ÊTRE PERDU** : les justificatifs de
la fabrique Matter y vivent. Sans ce dossier, le robot est à réappairer
entièrement depuis l'application Ecovacs.

## Essais

```bash
./.venv/bin/python essais.py     # sans robot, sans serveur Matter, sans kernel
```

## Ce que le matériel impose (appris à l'usage)

- **Pour lancer** : `SelectAreas` (grappe 336), puis un mode de nettoyage
  compatible avec l'état de la station, puis `ChangeToMode → Cleaning`.
- **Pour arrêter** : `ChangeToMode → Idle` **d'abord** (ce qui clôt la tâche),
  **puis** `GoHome`. Un `GoHome` seul le fait rentrer, mais le mode restant
  `Cleaning`, il juge sa tâche inachevée et **repart**. D'où deux routes :
  `stop_and_dock` (franc) et `dock` (qui le dit dans sa réponse).
- **Ne pas lancer un mode à serpillère pendant un séchage** : les ordres sont
  acceptés, et le robot aspire **sur place** sans quitter la base — ses
  serpillères sont au séchoir. Le plugin bascule alors en `Vacuum-Only` et le
  signale dans la réponse.
- **Le mode de marche ment sur la disponibilité** : il dit `Idle` pendant les
  quatre heures de séchage. C'est la **phase** (`Dry Mop`) qui dit la vérité.
- **La sélection de pièces survit** à la fin du nettoyage. La vider avant un
  nouveau lancement, sinon il refait la même pièce — d'où `clear_selection`,
  et un `SelectAreas` systématique avant chaque départ.
- **Pas de temps restant** : Matter prévoit `CountdownTime`, Ecovacs ne
  l'implémente pas. Le séchage s'horodate soi-même, et l'estimation est
  étiquetée comme telle.
- **Aucune commande de station** : ni interrompre le séchage, ni vidanger, ni
  laver. L'état des consommables remonte uniquement par le code d'erreur
  (`1/97/5`) : bac plein, réservoir vide, serpillère absente.
- **La grappe Alimentation est sur le point d'extrémité 1**, avec le robot —
  pas sur le 0, où l'on conclut à tort qu'il n'y a pas de batterie.
