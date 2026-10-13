"""symbion-plugin-lights — point d'entrée.

Lance, dans l'ordre : les pilotes (un fil par lampe), le bus MQTT, l'API sur
socket Unix, puis l'enregistrement auprès du kernel. L'enregistrement vient en
DERNIER : le kernel vérifie que la socket existe avant d'accepter, et il a
raison — une route enregistrée vers une socket absente est une panne silencieuse.

    python -m src [chemin/vers/lights.toml]
"""
from __future__ import annotations

import asyncio
import copy
import os
import signal
import sys
from pathlib import Path

_ici = Path(__file__).resolve().parent
sys.path.insert(0, str(_ici))
# modules communs aux plugins Python (pendant de symbion-plugin-common en Rust)
sys.path.insert(0, str(_ici.parent.parent / "symbion-plugin-pycommon"))

from aiohttp import web

import api
import config as config_mod
from bus_mqtt import BusMqtt
from noyau import Noyau
from pilote_tuya import PiloteTuya
from registre import Registre

VERSION = "1.0.0"
NOM = "lights"


def actions_avec_pieces(pieces: list[str], scenes: list[str]) -> list[dict]:
    """Remplit les listes déroulantes du rule builder avec les vraies pièces.

    Sans ça le porteur doit taper « cuisine » à la main dans l'éditeur de
    règles, et une faute de frappe donne une règle qui ne fait rien sans
    jamais le dire."""
    actions = copy.deepcopy(api.ACTIONS)
    for action in actions:
        for param in action["params"]:
            if param["name"] == "room" and pieces:
                param["options"] = [{"value": p, "label": p.capitalize()} for p in sorted(pieces)]
            if param["name"] == "scene" and scenes:
                param["options"] = [{"value": s, "label": s} for s in sorted(scenes)]
    return actions


async def main() -> int:
    chemin = Path(sys.argv[1]) if len(sys.argv) > 1 else config_mod.CHEMIN_DEFAUT
    try:
        conf = config_mod.charger(chemin)
    except config_mod.ConfigInvalide as e:
        print(f"[lights] {e}", flush=True)
        return 1

    racine = Path(conf["kernel"]["root"])
    bus = BusMqtt(conf["mqtt"]["host"], conf["mqtt"]["port"], NOM)
    noyau = Noyau(conf["kernel"]["url"], NOM, racine)
    registre = Registre(bus, noyau, racine / conf["state"]["scenes_path"])
    registre.armer(asyncio.get_running_loop())

    for lampe_conf in conf["lights"]:
        lampe = registre.declarer(lampe_conf)
        pilote = PiloteTuya(lampe_conf, registre.depuis_pilote)
        registre.pilotes[lampe.id] = pilote
        pilote.start()
        print(f"[lights] pilote lancé : {lampe.name} ({lampe.room})", flush=True)

    bus.demarrer()
    bus.publier(
        "symbion/plugins/lights/manifest",
        {
            "plugin": NOM,
            "version": VERSION,
            "routes": api.ROUTES,
            "actions": [a["name"] for a in api.ACTIONS],
            "lights": [l.id for l in registre.lampes.values()],
            "rooms": sorted(registre.pieces()),
        },
        retain=True,
    )
    bus.publier(
        "symbion/plugins/lights/status",
        {"plugin": NOM, "status": "starting", "version": VERSION},
        retain=True,
    )

    chemin_socket = conf["http"]["socket_path"]
    Path(chemin_socket).parent.mkdir(parents=True, exist_ok=True)
    if Path(chemin_socket).exists():
        Path(chemin_socket).unlink()
    coureur = web.AppRunner(api.construire(registre, VERSION), access_log=None)
    await coureur.setup()
    site = web.UnixSite(coureur, chemin_socket)
    await site.start()
    # Le kernel tourne sous le même compte, mais un plugin lancé en root
    # écrirait une socket que le kernel ne pourrait pas ouvrir.
    os.chmod(chemin_socket, 0o666)
    print(f"[lights] API sur {chemin_socket}", flush=True)

    await noyau.enregistrer(
        socket_path=chemin_socket,
        routes=api.ROUTES,
        actions=actions_avec_pieces(sorted(registre.pieces()), list(registre.scenes)),
        version=VERSION,
        description="Lampes Tuya pilotées en local (LAN, sans cloud)",
    )
    bus.publier(
        "symbion/plugins/lights/status",
        {"plugin": NOM, "status": "healthy", "version": VERSION},
        retain=True,
    )
    # L'état de départ, pour que les conditions des règles aient de quoi lire
    # avant le premier changement de lampe.
    registre.publier_tout()

    fin = asyncio.Event()
    boucle = asyncio.get_running_loop()
    for sig in (signal.SIGTERM, signal.SIGINT):
        boucle.add_signal_handler(sig, fin.set)
    print("[lights] prêt", flush=True)
    await fin.wait()

    print("[lights] arrêt demandé", flush=True)
    for pilote in registre.pilotes.values():
        pilote.arreter()
    bus.arreter()
    await coureur.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
