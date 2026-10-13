"""API HTTP sur socket Unix, proxifiée par le kernel en /v1/plugin-api/lights/*.

Deux familles de routes :
  · REST (`/lights`, `/scenes`) pour la PWA et le porteur ;
  · `/actions/*` en POST pour les automations — c'est la forme que
    `plugin_command` sait appeler, et celle que le rule builder présente
    sous forme de formulaire au lieu d'un payload JSON libre.
"""
from __future__ import annotations

import time

from aiohttp import web

from modele import Commande, Scene

DEMARRE = time.time()


def construire(registre, version: str) -> web.Application:
    app = web.Application()

    # -- santé --------------------------------------------------------------

    async def sante(_req: web.Request) -> web.Response:
        lampes = list(registre.lampes.values())
        en_ligne = [l for l in lampes if l.online]
        # Un plugin « healthy » dont aucune lampe ne répond est un mensonge :
        # on dégrade dès qu'une lampe déclarée est muette.
        etat = "healthy" if len(en_ligne) == len(lampes) else ("degraded" if en_ligne else "unhealthy")
        return web.json_response({
            "plugin": "lights",
            "status": etat,
            "version": version,
            "uptime_seconds": round(time.time() - DEMARRE),
            "lights_total": len(lampes),
            "lights_online": len(en_ligne),
            "offline": [{"id": l.id, "last_error": l.last_error} for l in lampes if not l.online],
        })

    # -- lampes -------------------------------------------------------------

    async def lister(_req: web.Request) -> web.Response:
        return web.json_response(registre.resume())

    async def une(req: web.Request) -> web.Response:
        lampe = registre.lampes.get(req.match_info["light_id"])
        if lampe is None:
            return web.json_response({"error": "lampe inconnue"}, status=404)
        return web.json_response(lampe.vue())

    async def corps_json(req: web.Request) -> dict:
        if not req.can_read_body:
            return {}
        try:
            donnees = await req.json()
        except Exception:                                # noqa: BLE001
            return {}
        return donnees if isinstance(donnees, dict) else {}

    async def commander(req: web.Request) -> web.Response:
        light_id = req.match_info["light_id"]
        if light_id not in registre.lampes:
            return web.json_response({"error": "lampe inconnue"}, status=404)
        try:
            cmd = Commande.depuis_json(await corps_json(req))
        except (TypeError, ValueError) as e:
            return web.json_response({"error": f"corps invalide : {e}"}, status=400)
        if cmd.vide():
            return web.json_response({"error": "rien à faire : on, brightness ou color_temp_k"}, status=400)
        if not registre.commander(light_id, cmd):
            return web.json_response({"error": "pilote absent pour cette lampe"}, status=503)
        return web.json_response({"status": "accepté", "light_id": light_id})

    async def commander_piece(req: web.Request) -> web.Response:
        piece = req.match_info["room"]
        try:
            cmd = Commande.depuis_json(await corps_json(req))
        except (TypeError, ValueError) as e:
            return web.json_response({"error": f"corps invalide : {e}"}, status=400)
        if cmd.vide():
            return web.json_response({"error": "rien à faire"}, status=400)
        touchees = registre.commander_piece(piece, cmd)
        if not touchees:
            return web.json_response({"error": f"aucune lampe dans « {piece} »"}, status=404)
        return web.json_response({"status": "accepté", "room": piece, "lights": touchees})

    # -- scènes -------------------------------------------------------------

    async def lister_scenes(_req: web.Request) -> web.Response:
        return web.json_response({"scenes": [s.vue() for s in registre.scenes.values()]})

    async def creer_scene(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        scene_id = (corps.get("id") or "").strip()
        cibles = corps.get("targets")
        if not scene_id or not isinstance(cibles, list) or not cibles:
            return web.json_response({"error": "il faut un id et des targets non vides"}, status=400)
        inconnues = [c.get("light_id") for c in cibles if c.get("light_id") not in registre.lampes]
        if inconnues:
            return web.json_response(
                {"error": f"lampes inconnues : {', '.join(str(i) for i in inconnues)}"}, status=400
            )
        registre.enregistrer_scene(Scene(id=scene_id, name=corps.get("name") or scene_id, targets=cibles))
        return web.json_response({"status": "enregistrée", "id": scene_id})

    async def supprimer_scene(req: web.Request) -> web.Response:
        if not registre.supprimer_scene(req.match_info["scene_id"]):
            return web.json_response({"error": "scène inconnue"}, status=404)
        return web.json_response({"status": "supprimée"})

    async def activer_scene(req: web.Request) -> web.Response:
        touchees = registre.activer_scene(req.match_info["scene_id"])
        if touchees is None:
            return web.json_response({"error": "scène inconnue"}, status=404)
        return web.json_response({"status": "activée", "lights": touchees})

    # -- actions pour les automations ---------------------------------------

    async def action_piece(req: web.Request, allumer: bool) -> web.Response:
        corps = await corps_json(req)
        piece = (corps.get("room") or corps.get("piece") or "").strip()
        if not piece:
            return web.json_response({"error": "paramètre « room » attendu"}, status=400)
        touchees = registre.commander_piece(piece, Commande(on=allumer))
        if not touchees:
            return web.json_response({"error": f"aucune lampe dans « {piece} »"}, status=404)
        return web.json_response({"status": "accepté", "room": piece, "lights": touchees})

    async def action_room_on(req: web.Request) -> web.Response:
        return await action_piece(req, True)

    async def action_room_off(req: web.Request) -> web.Response:
        return await action_piece(req, False)

    async def action_all_off(_req: web.Request) -> web.Response:
        return web.json_response({"status": "accepté", "lights": registre.commander_tout(Commande(on=False))})

    async def action_set_scene(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        scene_id = (corps.get("scene") or corps.get("id") or "").strip()
        if not scene_id:
            return web.json_response({"error": "paramètre « scene » attendu"}, status=400)
        touchees = registre.activer_scene(scene_id)
        if touchees is None:
            return web.json_response({"error": "scène inconnue"}, status=404)
        return web.json_response({"status": "activée", "scene": scene_id, "lights": touchees})

    async def action_set_light(req: web.Request) -> web.Response:
        corps = await corps_json(req)
        light_id = (corps.get("light_id") or "").strip()
        if light_id not in registre.lampes:
            return web.json_response({"error": "paramètre « light_id » inconnu"}, status=404)
        cmd = Commande.depuis_json(corps)
        if cmd.vide():
            return web.json_response({"error": "rien à faire"}, status=400)
        registre.commander(light_id, cmd)
        return web.json_response({"status": "accepté", "light_id": light_id})

    app.add_routes([
        web.get("/health", sante),
        web.get("/lights", lister),
        web.get("/lights/groups", lister),
        web.put("/lights/groups/{room}", commander_piece),
        web.get("/lights/{light_id}", une),
        web.put("/lights/{light_id}", commander),
        web.get("/scenes", lister_scenes),
        web.post("/scenes", creer_scene),
        web.delete("/scenes/{scene_id}", supprimer_scene),
        web.put("/scenes/{scene_id}/activate", activer_scene),
        web.post("/scenes/{scene_id}/activate", activer_scene),
        web.post("/actions/room_on", action_room_on),
        web.post("/actions/room_off", action_room_off),
        web.post("/actions/all_off", action_all_off),
        web.post("/actions/set_scene", action_set_scene),
        web.post("/actions/set_light", action_set_light),
    ])
    return app


# Déclaré au kernel : le rule builder en fait des formulaires, et
# `plugin_command` les appelle sans que le porteur écrive de JSON.
ACTIONS = [
    {
        "name": "room_on", "label": "Allumer une pièce", "icon": "💡",
        "description": "Allume toutes les lampes d'une pièce",
        "route": "actions/room_on", "method": "POST", "impact_level": "Low",
        "params": [{"name": "room", "label": "Pièce", "type": "string", "required": True}],
    },
    {
        "name": "room_off", "label": "Éteindre une pièce", "icon": "🌑",
        "description": "Éteint toutes les lampes d'une pièce",
        "route": "actions/room_off", "method": "POST", "impact_level": "Low",
        "params": [{"name": "room", "label": "Pièce", "type": "string", "required": True}],
    },
    {
        "name": "all_off", "label": "Tout éteindre", "icon": "🌙",
        "description": "Éteint toutes les lampes de la maison",
        "route": "actions/all_off", "method": "POST", "impact_level": "Low",
        "params": [],
    },
    {
        "name": "set_scene", "label": "Activer une scène", "icon": "🎬",
        "description": "Applique une scène enregistrée",
        "route": "actions/set_scene", "method": "POST", "impact_level": "Low",
        "params": [{"name": "scene", "label": "Scène", "type": "string", "required": True}],
    },
    {
        "name": "set_light", "label": "Régler une lampe", "icon": "🔆",
        "description": "Allume, éteint ou règle une lampe précise",
        "route": "actions/set_light", "method": "POST", "impact_level": "Low",
        "params": [
            {"name": "light_id", "label": "Lampe", "type": "string", "required": True},
            {"name": "on", "label": "Allumée", "type": "boolean", "required": False},
            {"name": "brightness", "label": "Luminosité (%)", "type": "number", "required": False},
            {"name": "color_temp_k", "label": "Température (K)", "type": "number", "required": False},
        ],
    },
]

ROUTES = ["/health", "/lights", "/scenes", "/actions"]
