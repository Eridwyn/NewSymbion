"""API HTTP du plugin robot, proxifiée en /v1/plugin-api/robot/*.

Le pilotage suit une RECETTE, éprouvée le 12/10/2026 et encodée ici pour ne
plus jamais être à retrouver :

  POUR LANCER   1. SelectAreas (grappe 336) avec les identifiants de pièces
                2. un mode de nettoyage COMPATIBLE avec l'état de la station
                3. ChangeToMode → Cleaning (grappe 84)

  POUR ARRÊTER  l'inverse exact : ChangeToMode → Idle D'ABORD (ce qui CLÔT la
                tâche), PUIS GoHome. Un GoHome seul le fait rentrer, mais le
                mode de marche restant `Cleaning`, il juge sa tâche inachevée
                et REPART.

Et le garde-fou qui a coûté un essai raté : lancer un mode à serpillère
pendant que la station sèche fait aspirer le robot SUR PLACE, à la base. On
bascule alors en `Vacuum-Only` et on le dit dans la réponse.
"""
from __future__ import annotations

import asyncio
import time

from aiohttp import web

from matter_ws import (
    CLUSTER_CLEAN_MODE, CLUSTER_OPERATIONAL, CLUSTER_RUN_MODE, CLUSTER_SERVICE_AREA,
    CLEAN_MODES, MatterIndisponible,
)
from etat import MODES_AVEC_SERPILLERE, PHASE_SECHAGE, _libelle_mode

DEMARRE = time.time()


def construire(client, etat, version: str) -> web.Application:
    app = web.Application()

    async def corps_json(req: web.Request) -> dict:
        if not req.can_read_body:
            return {}
        try:
            d = await req.json()
        except Exception:                                # noqa: BLE001
            return {}
        return d if isinstance(d, dict) else {}

    def index_mode_nettoyage(nom: str) -> int | None:
        for entree in client.attributs.get(CLEAN_MODES) or []:
            if (entree.get("0") or "").strip().lower() == nom.strip().lower():
                return entree.get("1")
        return None

    def resoudre_zones(demandees: list) -> tuple[list[int], list[str]]:
        """Accepte des identifiants, des noms Ecovacs ou des pièces Symbion."""
        zones = etat.zones()
        par_nom = {n.strip().lower(): z for z, n in zones.items() if n}
        par_piece: dict[str, int] = {}
        for zid, nom in zones.items():
            piece = etat.piece_symbion(nom) if nom else None
            if piece:
                par_piece.setdefault(piece, zid)
        trouvees, inconnues = [], []
        for d in demandees:
            if isinstance(d, int) or (isinstance(d, str) and d.isdigit()):
                zid = int(d)
                (trouvees if zid in zones else inconnues).append(zid if zid in zones else str(d))
                continue
            cle = str(d).strip().lower()
            zid = par_nom.get(cle) or par_piece.get(cle)
            if zid is None:
                inconnues.append(str(d))
            else:
                trouvees.append(zid)
        return trouvees, inconnues

    # -- lectures -----------------------------------------------------------

    async def sante(_req: web.Request) -> web.Response:
        vue = etat.vue()
        return web.json_response({
            "plugin": "robot",
            "status": "healthy" if client.connecte else "unhealthy",
            "version": version,
            "uptime_seconds": round(time.time() - DEMARRE),
            "matter_connected": client.connecte,
            "run_mode": vue["run_mode"]["label"],
            "phase": vue["phase"]["label"],
            "battery": vue["battery"],
        })

    async def lire(_req: web.Request) -> web.Response:
        return web.json_response(etat.vue())

    async def lire_zones(_req: web.Request) -> web.Response:
        return web.json_response({"zones": etat.vue()["zones"]})

    # -- pilotage -----------------------------------------------------------

    async def lancer(zones_ids: list[int], mode_demande: str | None) -> dict:
        vue = etat.vue()
        notes: list[str] = []

        mode = mode_demande or vue["clean_mode"]["label"] or "Vacuum & Mop"
        if vue["phase"]["label"] == PHASE_SECHAGE and mode in MODES_AVEC_SERPILLERE:
            notes.append(
                f"station en séchage : « {mode} » ferait aspirer le robot sur place, "
                "basculé en « Vacuum-Only »"
            )
            mode = "Vacuum-Only"

        # 1. les pièces (la sélection SURVIT à la fin du nettoyage : on la
        #    réécrit toujours, sinon il refait la précédente)
        await client.commander(CLUSTER_SERVICE_AREA, "SelectAreas", {"newAreas": zones_ids})

        # 2. le mode de nettoyage
        index = index_mode_nettoyage(mode)
        if index is None:
            return {"error": f"mode de nettoyage inconnu : {mode}"}
        if index != vue["clean_mode"]["index"]:
            await client.commander(CLUSTER_CLEAN_MODE, "ChangeToMode", {"newMode": index})

        # 3. en route
        await client.commander(CLUSTER_RUN_MODE, "ChangeToMode", {"newMode": 1})
        return {
            "status": "lancé",
            "zones": zones_ids,
            "clean_mode": mode,
            "notes": notes,
        }

    async def action_clean_rooms(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        demandees = corps.get("rooms") or corps.get("zones") or corps.get("pieces")
        if isinstance(demandees, str):
            demandees = [p for p in demandees.split(",") if p.strip()]
        if not demandees:
            return web.json_response({"error": "paramètre « rooms » attendu"}, status=400)
        ids, inconnues = resoudre_zones(demandees)
        if inconnues:
            return web.json_response(
                {"error": f"pièces inconnues : {', '.join(inconnues)}",
                 "connues": [z["name"] for z in etat.vue()["zones"]]}, status=400)
        try:
            resultat = await lancer(ids, corps.get("mode"))
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response(resultat, status=400 if "error" in resultat else 200)

    async def action_clean_all(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        ids = [z["id"] for z in etat.vue()["zones"] if z["name"]]
        if not ids:
            return web.json_response({"error": "aucune pièce connue du robot"}, status=503)
        try:
            resultat = await lancer(ids, corps.get("mode"))
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response(resultat, status=400 if "error" in resultat else 200)

    async def action_stop_and_dock(_req: web.Request) -> web.Response:
        """Clore la tâche AVANT de renvoyer, sinon il repart."""
        try:
            await client.commander(CLUSTER_RUN_MODE, "ChangeToMode", {"newMode": 0})
            await asyncio.sleep(2)
            reponse = await client.commander(CLUSTER_OPERATIONAL, "GoHome")
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({"status": "tâche close puis retour à la base", "matter": reponse})

    async def action_dock(_req: web.Request) -> web.Response:
        try:
            reponse = await client.commander(CLUSTER_OPERATIONAL, "GoHome")
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({
            "status": "retour demandé",
            "avertissement": "le mode de marche n'a pas été clos : s'il était en nettoyage, il repartira. "
                             "Utiliser stop_and_dock pour un arrêt franc.",
            "matter": reponse,
        })

    async def action_set_clean_mode(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        mode = (corps.get("mode") or "").strip()
        index = index_mode_nettoyage(mode) if mode else None
        if index is None:
            return web.json_response(
                {"error": "paramètre « mode » inconnu",
                 "connus": [e.get("0") for e in client.attributs.get(CLEAN_MODES) or []]}, status=400)
        try:
            await client.commander(CLUSTER_CLEAN_MODE, "ChangeToMode", {"newMode": index})
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({"status": "mode réglé", "mode": mode})

    async def action_clear_selection(_req: web.Request) -> web.Response:
        try:
            await client.commander(CLUSTER_SERVICE_AREA, "SelectAreas", {"newAreas": []})
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({"status": "sélection vidée"})

    async def action_pause(_req: web.Request) -> web.Response:
        try:
            reponse = await client.commander(CLUSTER_OPERATIONAL, "Pause")
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({
            "status": "pause demandée",
            "avertissement": "la station refuse la pause pendant un séchage (errorStateID 3)",
            "matter": reponse,
        })

    async def action_resume(_req: web.Request) -> web.Response:
        try:
            reponse = await client.commander(CLUSTER_OPERATIONAL, "Resume")
        except MatterIndisponible as e:
            return web.json_response({"error": str(e)}, status=503)
        return web.json_response({"status": "reprise demandée", "matter": reponse})

    app.add_routes([
        web.get("/health", sante),
        web.get("/robot", lire),
        web.get("/robot/zones", lire_zones),
        web.post("/actions/clean_rooms", action_clean_rooms),
        web.post("/actions/clean_all", action_clean_all),
        web.post("/actions/stop_and_dock", action_stop_and_dock),
        web.post("/actions/dock", action_dock),
        web.post("/actions/set_clean_mode", action_set_clean_mode),
        web.post("/actions/clear_selection", action_clear_selection),
        web.post("/actions/pause", action_pause),
        web.post("/actions/resume", action_resume),
    ])
    return app


ACTIONS = [
    {
        "name": "clean_rooms", "label": "Nettoyer des pièces", "icon": "🧹",
        "description": "Sélectionne les pièces puis lance le nettoyage",
        "route": "actions/clean_rooms", "method": "POST", "impact_level": "Medium",
        "params": [
            {"name": "rooms", "label": "Pièces (séparées par des virgules)", "type": "string", "required": True},
            {"name": "mode", "label": "Mode de nettoyage", "type": "string", "required": False},
        ],
    },
    {
        "name": "clean_all", "label": "Nettoyer toute la maison", "icon": "🏠",
        "description": "Lance le nettoyage de toutes les pièces cartographiées",
        "route": "actions/clean_all", "method": "POST", "impact_level": "Medium",
        "params": [{"name": "mode", "label": "Mode de nettoyage", "type": "string", "required": False}],
    },
    {
        "name": "stop_and_dock", "label": "Arrêter et renvoyer à la base", "icon": "🛑",
        "description": "Clôt la tâche puis renvoie — le seul arrêt franc",
        "route": "actions/stop_and_dock", "method": "POST", "impact_level": "Low",
        "params": [],
    },
    {
        "name": "set_clean_mode", "label": "Changer le mode de nettoyage", "icon": "💧",
        "description": "Vacuum & Mop · Vacuum-Only · Mopping · Mop after Vacuum",
        "route": "actions/set_clean_mode", "method": "POST", "impact_level": "Low",
        "params": [{
            "name": "mode", "label": "Mode", "type": "string", "required": True,
            "options": [
                {"value": "Vacuum & Mop", "label": "Aspiration + serpillère"},
                {"value": "Vacuum-Only", "label": "Aspiration seule"},
                {"value": "Mopping", "label": "Serpillère seule"},
                {"value": "Mop after Vacuum", "label": "Serpillère après aspiration"},
            ],
        }],
    },
    {
        "name": "clear_selection", "label": "Vider la sélection de pièces", "icon": "🧼",
        "description": "La sélection survit au nettoyage : la vider évite qu'il refasse la même pièce",
        "route": "actions/clear_selection", "method": "POST", "impact_level": "Low",
        "params": [],
    },
]

ROUTES = ["/health", "/robot", "/actions"]
