"""Registre : l'état vivant des lampes, et tout ce qui en découle.

C'est le seul endroit qui décide « ça a changé » — et un changement déclenche
trois choses : l'état MQTT, les features pour les conditions d'automation, et
l'événement poussé au kernel pour en RÉVEILLER une.

Rien n'est publié si rien n'a changé : une lampe pousse son état plusieurs
fois par minute, et une automation qui se redéclencherait à chaque poussée
rallumerait la pièce que le porteur vient d'éteindre.
"""
from __future__ import annotations

import asyncio
import json
from pathlib import Path

from modele import Capacites, Commande, Lampe, Scene


class Registre:
    def __init__(self, bus, noyau, chemin_scenes: Path) -> None:
        self.bus = bus
        self.noyau = noyau
        self.chemin_scenes = chemin_scenes
        self.lampes: dict[str, Lampe] = {}
        self.scenes: dict[str, Scene] = {}
        self.pilotes: dict[str, object] = {}
        self._boucle: asyncio.AbstractEventLoop | None = None
        self._charger_scenes()

    # -- montage ------------------------------------------------------------

    def armer(self, boucle: asyncio.AbstractEventLoop) -> None:
        self._boucle = boucle

    def publier_tout(self) -> None:
        """Publie l'état complet, au démarrage.

        Une `Condition::Feature` sur une feature ABSENTE est évaluée fausse
        (« not found in registry ») : sans cette première publication, une
        règle d'extinction resterait muette jusqu'au premier changement de
        lampe — et ce changement peut ne jamais venir."""
        for lampe in self.lampes.values():
            self.bus.publier(f"symbion/lights/{lampe.id}", lampe.vue(), retain=True)
            self.bus.feature(f"lights.{lampe.id}.on", lampe.on)
            self.bus.feature(f"lights.{lampe.id}.brightness", lampe.brightness)
        for piece in self.pieces():
            self.bus.feature(f"lights.{piece}.any_on", self.piece_allumee(piece))
        self.bus.feature("lights.any_on", self.quelque_chose_allume())
        self.bus.feature("lights.rooms_on", self.pieces_allumees())
        self.bus.publier("symbion/lights/summary", self.resume(), retain=True)

    def declarer(self, conf: dict) -> Lampe:
        lampe = Lampe(
            id=conf["id"],
            name=conf.get("name") or conf["id"],
            room=(conf.get("room") or "sans-piece").strip().lower(),
            driver=conf.get("driver", "tuya"),
            caps=Capacites(
                dimmable=bool(conf.get("dimmable", True)),
                color_temp=bool(conf.get("color_temp", True)),
            ),
        )
        self.lampes[lampe.id] = lampe
        return lampe

    # -- entrée depuis les pilotes (appelée DEPUIS UN AUTRE FIL) ------------

    def depuis_pilote(self, light_id: str, maj: dict) -> None:
        """Pont fil pilote → boucle asyncio. Toujours sûr d'appeler."""
        if self._boucle is None:
            return
        self._boucle.call_soon_threadsafe(self._appliquer, light_id, maj)

    def _appliquer(self, light_id: str, maj: dict) -> None:
        lampe = self.lampes.get(light_id)
        if lampe is None:
            return
        avant = (lampe.online, lampe.on, lampe.brightness, lampe.color_temp_k)
        piece_avant = self.piece_allumee(lampe.room)

        for cle, valeur in maj.items():
            if hasattr(lampe, cle):
                setattr(lampe, cle, valeur)

        apres = (lampe.online, lampe.on, lampe.brightness, lampe.color_temp_k)
        if avant == apres:
            return

        piece_apres = self.piece_allumee(lampe.room)
        asyncio.ensure_future(self._diffuser(lampe, avant, piece_avant, piece_apres))

    async def _diffuser(self, lampe: Lampe, avant: tuple, piece_avant: bool, piece_apres: bool) -> None:
        self.bus.publier(f"symbion/lights/{lampe.id}", lampe.vue(), retain=True)
        self.bus.publier("symbion/lights/summary", self.resume(), retain=True)
        self.bus.feature(f"lights.{lampe.id}.on", lampe.on)
        self.bus.feature(f"lights.{lampe.id}.brightness", lampe.brightness)
        self.bus.feature(f"lights.{lampe.room}.any_on", piece_apres)
        # « Reste-t-il quelque chose à éteindre ? » — c'est ce qui permet à une
        # règle d'extinction de ne PAS tourner à vide : sans elle, « personne à
        # la maison → tout éteindre » s'exécuterait toutes les minutes, à
        # chaque fois pour rien, et saturerait l'historique d'exécutions.
        self.bus.feature("lights.any_on", self.quelque_chose_allume())
        self.bus.feature("lights.rooms_on", self.pieces_allumees())

        # idem que pour le robot : features acquittées avant l'événement
        self.bus.attendre_remise()

        await self.noyau.pousser(
            "light_changed",
            {
                "light_id": lampe.id,
                "room": lampe.room,
                "on": lampe.on,
                "online": lampe.online,
                "brightness": lampe.brightness,
                "color_temp_k": lampe.color_temp_k,
                "was_on": avant[1],
            },
        )
        if piece_avant != piece_apres:
            await self.noyau.pousser(
                "room_any_on_changed",
                {"room": lampe.room, "any_on": piece_apres},
            )

    # -- lectures -----------------------------------------------------------

    def piece_allumee(self, piece: str) -> bool:
        return any(l.on and l.online for l in self.lampes.values() if l.room == piece)

    def quelque_chose_allume(self) -> bool:
        return any(l.on and l.online for l in self.lampes.values())

    def pieces_allumees(self) -> list[str]:
        """Les pièces où quelque chose est allumé — une vraie liste, donc une
        `StringList` côté kernel, interrogeable par `contains`."""
        return sorted({l.room for l in self.lampes.values() if l.on and l.online})

    def pieces(self) -> dict[str, list[str]]:
        par_piece: dict[str, list[str]] = {}
        for l in self.lampes.values():
            par_piece.setdefault(l.room, []).append(l.id)
        return par_piece

    def resume(self) -> dict:
        return {
            "lights": [l.vue() for l in self.lampes.values()],
            "rooms": {p: self.piece_allumee(p) for p in self.pieces()},
            "scenes": [s.id for s in self.scenes.values()],
        }

    # -- écritures ----------------------------------------------------------

    def commander(self, light_id: str, cmd: Commande) -> bool:
        pilote = self.pilotes.get(light_id)
        if pilote is None:
            return False
        pilote.envoyer(cmd)
        return True

    def commander_piece(self, piece: str, cmd: Commande) -> list[str]:
        piece = piece.strip().lower()
        touchees = [l.id for l in self.lampes.values() if l.room == piece]
        for light_id in touchees:
            self.commander(light_id, cmd)
        return touchees

    def commander_tout(self, cmd: Commande) -> list[str]:
        for light_id in self.lampes:
            self.commander(light_id, cmd)
        return list(self.lampes)

    # -- scènes -------------------------------------------------------------

    def _charger_scenes(self) -> None:
        try:
            brut = json.loads(self.chemin_scenes.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return
        for s in brut.get("scenes", []):
            try:
                self.scenes[s["id"]] = Scene(id=s["id"], name=s.get("name") or s["id"], targets=s.get("targets") or [])
            except KeyError:
                continue

    def _ecrire_scenes(self) -> None:
        self.chemin_scenes.parent.mkdir(parents=True, exist_ok=True)
        provisoire = self.chemin_scenes.with_suffix(".tmp")
        provisoire.write_text(
            json.dumps({"scenes": [s.vue() for s in self.scenes.values()]}, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        provisoire.replace(self.chemin_scenes)

    def enregistrer_scene(self, scene: Scene) -> None:
        self.scenes[scene.id] = scene
        self._ecrire_scenes()

    def supprimer_scene(self, scene_id: str) -> bool:
        if self.scenes.pop(scene_id, None) is None:
            return False
        self._ecrire_scenes()
        return True

    def activer_scene(self, scene_id: str) -> list[str] | None:
        scene = self.scenes.get(scene_id)
        if scene is None:
            return None
        touchees = []
        for cible in scene.targets:
            light_id = cible.get("light_id")
            if light_id not in self.lampes:
                continue
            cmd = Commande(
                on=cible.get("on"),
                brightness=cible.get("brightness"),
                color_temp_k=cible.get("color_temp_k"),
            )
            if self.commander(light_id, cmd):
                touchees.append(light_id)
        return touchees
