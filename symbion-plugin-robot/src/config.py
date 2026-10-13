"""Configuration du plugin robot."""
from __future__ import annotations

import tomllib
from pathlib import Path

CHEMIN_DEFAUT = Path("/opt/symbion/config/robot.toml")


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

    matter = brut.get("matter") or {}
    mqtt = brut.get("mqtt") or {}
    http = brut.get("http") or {}
    kernel = brut.get("kernel") or {}
    robot = brut.get("robot") or {}
    return {
        "matter": {
            "url": matter.get("url", "ws://127.0.0.1:5580/ws"),
            "node_id": int(matter.get("node_id", 1)),
        },
        "mqtt": {"host": mqtt.get("host", "127.0.0.1"), "port": int(mqtt.get("port", 1883))},
        "http": {"socket_path": http.get("socket_path", "/run/symbion-plugins/robot.sock")},
        "kernel": {
            "url": kernel.get("url", "https://127.0.0.1:8443"),
            "root": kernel.get("root", "/home/eridwyn/RustroverProjects/NewSymbion"),
        },
        "robot": {
            "rooms": brut.get("rooms") or {},
            "drying_nominal_minutes": int(robot.get("drying_nominal_minutes", 240)),
            "battery_low_threshold": int(robot.get("battery_low_threshold", 20)),
        },
    }
