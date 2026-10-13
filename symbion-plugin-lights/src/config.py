"""Lecture de la configuration TOML.

Les `local_key` sont des SECRETS : le fichier vit hors du dépôt
(`/opt/symbion/config/lights.toml`, en 0600) comme pour synology et ssl. Le
gabarit commité ne contient que des valeurs factices.
"""
from __future__ import annotations

import tomllib
from pathlib import Path

CHEMIN_DEFAUT = Path("/opt/symbion/config/lights.toml")

CHAMPS_LAMPE = ("id", "device_id", "local_key", "ip")


class ConfigInvalide(Exception):
    pass


def charger(chemin: Path | None = None) -> dict:
    chemin = chemin or CHEMIN_DEFAUT
    try:
        brut = tomllib.loads(chemin.read_text(encoding="utf-8"))
    except OSError as e:
        raise ConfigInvalide(f"configuration illisible ({chemin}) : {e}") from e
    except tomllib.TOMLDecodeError as e:
        raise ConfigInvalide(f"configuration mal formée ({chemin}) : {e}") from e

    lampes = brut.get("lights") or []
    if not lampes:
        raise ConfigInvalide(f"aucune lampe déclarée dans {chemin} — voir lights.example.toml")

    vus = set()
    for lampe in lampes:
        manquants = [c for c in CHAMPS_LAMPE if not lampe.get(c)]
        if manquants:
            raise ConfigInvalide(
                f"lampe {lampe.get('id') or '?'} : champs manquants {', '.join(manquants)}"
            )
        if lampe["id"] in vus:
            raise ConfigInvalide(f"identifiant de lampe en double : {lampe['id']}")
        vus.add(lampe["id"])

    mqtt = brut.get("mqtt") or {}
    http = brut.get("http") or {}
    kernel = brut.get("kernel") or {}
    return {
        "lights": lampes,
        "mqtt": {"host": mqtt.get("host", "127.0.0.1"), "port": int(mqtt.get("port", 1883))},
        "http": {"socket_path": http.get("socket_path", "/run/symbion-plugins/lights.sock")},
        "kernel": {
            "url": kernel.get("url", "https://127.0.0.1:8443"),
            "root": kernel.get("root", "/home/eridwyn/RustroverProjects/NewSymbion"),
        },
        "state": {
            "scenes_path": (brut.get("state") or {}).get("scenes_path", "data/lights-scenes.json")
        },
    }
