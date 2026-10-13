"""Modèle et normalisation — la seule partie qui connaît les unités Tuya.

Tout le reste du plugin parle en unités Symbion : luminosité en POURCENT,
température de couleur en KELVIN. Les échelles Tuya (10-1000, 0-1000) ne
sortent pas de ce fichier ni du pilote.
"""
from __future__ import annotations

from dataclasses import dataclass, field, asdict
from typing import Any

# Bornes Symbion
KELVIN_CHAUD = 2200
KELVIN_FROID = 6500


def kelvin_vers_pct(kelvin: int) -> float:
    """Kelvin → pourcentage de « froideur » (0 = chaud, 100 = froid).

    tinytuya expose la température en pourcentage de l'échelle de l'ampoule,
    et cette échelle diffère selon le type d'ampoule (0-255 ou 0-1000). On ne
    touche donc jamais aux unités Tuya ici : on reste en pourcentage, et
    tinytuya le ramène à l'échelle du matériel.
    """
    kelvin = max(KELVIN_CHAUD, min(KELVIN_FROID, int(kelvin)))
    return (kelvin - KELVIN_CHAUD) / (KELVIN_FROID - KELVIN_CHAUD) * 100.0


def pct_vers_kelvin(pct: float | None) -> int | None:
    if pct is None:
        return None
    pct = max(0.0, min(100.0, float(pct)))
    return round(KELVIN_CHAUD + (pct / 100.0) * (KELVIN_FROID - KELVIN_CHAUD))


def borne_pct(valeur) -> int | None:
    """Ramène une luminosité dans 1-100. 0 n'existe pas chez Tuya : une lampe
    à 0 % est une lampe éteinte, et c'est `on=false` qui le dit."""
    if valeur is None:
        return None
    try:
        return max(1, min(100, round(float(valeur))))
    except (TypeError, ValueError):
        return None


@dataclass
class Capacites:
    dimmable: bool = True
    color_temp: bool = True


@dataclass
class Lampe:
    """Une lampe telle que le reste de Symbion la voit."""

    id: str
    name: str
    room: str
    driver: str = "tuya"
    caps: Capacites = field(default_factory=Capacites)
    # état live, rempli par le pilote
    online: bool = False
    on: bool = False
    brightness: int = 0
    color_temp_k: int | None = None
    # dernière erreur rencontrée par le pilote, pour que /health ne mente pas
    last_error: str | None = None

    def vue(self) -> dict:
        d = asdict(self)
        d["caps"] = asdict(self.caps)
        return d


@dataclass
class Scene:
    """Un état visé pour un ensemble de lampes."""

    id: str
    name: str
    targets: list[dict]  # {light_id, on, brightness?, color_temp_k?}

    def vue(self) -> dict:
        return asdict(self)


@dataclass
class Commande:
    """Une commande normalisée adressée à une lampe."""

    on: bool | None = None
    brightness: int | None = None
    color_temp_k: int | None = None

    def vide(self) -> bool:
        return self.on is None and self.brightness is None and self.color_temp_k is None

    @staticmethod
    def depuis_json(corps: dict) -> "Commande":
        def entier(cle: str) -> int | None:
            v = corps.get(cle)
            if v is None:
                return None
            return int(v)

        on = corps.get("on")
        if isinstance(on, str):
            on = on.strip().lower() in ("1", "true", "on", "oui", "yes")
        return Commande(
            on=None if on is None else bool(on),
            brightness=entier("brightness"),
            color_temp_k=entier("color_temp_k"),
        )
