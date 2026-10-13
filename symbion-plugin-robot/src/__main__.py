"""symbion-plugin-robot — point d'entrée.

    python -m src [chemin/vers/robot.toml]
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
sys.path.insert(0, str(_ici.parent.parent / "symbion-plugin-pycommon"))

from aiohttp import web

import api
import config as config_mod
from bus_mqtt import BusMqtt
from etat import Etat
from matter_ws import ClientMatter
from noyau import Noyau

VERSION = "1.0.0"
NOM = "robot"


def actions_avec_pieces(zones: list[dict]) -> list[dict]:
    """Remplit la liste des pièces dans le rule builder avec les vraies.

    Les pièces viennent de la CARTE du robot : elles ne sont connues qu'une
    fois le lien Matter établi, pas au démarrage du processus."""
    actions = copy.deepcopy(api.ACTIONS)
    noms = [z["name"] for z in zones if z.get("name")]
    for action in actions:
        for param in action["params"]:
            if param["name"] == "rooms" and noms:
                param["placeholder"] = ", ".join(noms)
    return actions


async def main() -> int:
    chemin = Path(sys.argv[1]) if len(sys.argv) > 1 else config_mod.CHEMIN_DEFAUT
    try:
        conf = config_mod.charger(chemin)
    except config_mod.ConfigInvalide as e:
        print(f"[robot] {e}", flush=True)
        return 1

    racine = Path(conf["kernel"]["root"])
    bus = BusMqtt(conf["mqtt"]["host"], conf["mqtt"]["port"], NOM)
    noyau = Noyau(conf["kernel"]["url"], NOM, racine)

    etat_ref: dict = {}

    async def sur_attribut(chemin_attr: str, valeur) -> None:
        etat = etat_ref.get("etat")
        if etat is not None:
            await etat.sur_attribut(chemin_attr, valeur)

    client = ClientMatter(conf["matter"]["url"], conf["matter"]["node_id"], sur_attribut)
    etat = Etat(
        client, bus, noyau,
        pieces=conf["robot"]["rooms"],
        sechage_nominal=conf["robot"]["drying_nominal_minutes"],
        seuil_batterie=conf["robot"]["battery_low_threshold"],
    )
    etat_ref["etat"] = etat

    bus.demarrer()
    tache_matter = asyncio.create_task(client.tourner())

    chemin_socket = conf["http"]["socket_path"]
    Path(chemin_socket).parent.mkdir(parents=True, exist_ok=True)
    if Path(chemin_socket).exists():
        Path(chemin_socket).unlink()
    coureur = web.AppRunner(api.construire(client, etat, VERSION), access_log=None)
    await coureur.setup()
    await web.UnixSite(coureur, chemin_socket).start()
    os.chmod(chemin_socket, 0o666)
    print(f"[robot] API sur {chemin_socket}", flush=True)

    # On attend le lien Matter avant de s'enregistrer : sans lui on ne connaît
    # pas les pièces, et le rule builder afficherait une liste vide.
    try:
        await asyncio.wait_for(client._connecte.wait(), timeout=30)
        await etat.diffuser()
    except asyncio.TimeoutError:
        print("[robot] lien Matter pas encore établi, on s'enregistre quand même", flush=True)

    vue = etat.vue()
    bus.publier(
        "symbion/plugins/robot/manifest",
        {
            "plugin": NOM, "version": VERSION, "routes": api.ROUTES,
            "actions": [a["name"] for a in api.ACTIONS],
            "zones": vue["zones"],
        },
        retain=True,
    )
    await noyau.enregistrer(
        socket_path=chemin_socket,
        routes=api.ROUTES,
        actions=actions_avec_pieces(vue["zones"]),
        version=VERSION,
        description="Aspirateur Deebot T50 Pro Omni piloté en Matter local",
    )
    bus.publier(
        "symbion/plugins/robot/status",
        {"plugin": NOM, "status": "healthy" if client.connecte else "degraded", "version": VERSION},
        retain=True,
    )

    fin = asyncio.Event()
    boucle = asyncio.get_running_loop()
    for sig in (signal.SIGTERM, signal.SIGINT):
        boucle.add_signal_handler(sig, fin.set)
    print("[robot] prêt", flush=True)
    await fin.wait()

    print("[robot] arrêt demandé", flush=True)
    tache_matter.cancel()
    bus.arreter()
    await coureur.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
