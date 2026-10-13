"""Lien WebSocket avec python-matter-server.

Deux leçons sont gravées ici :

1. **`start_listening` n'est pas optionnel.** Le serveur ne pousse ses
   `attribute_updated` qu'aux clients qui l'ont demandé. Une connexion
   simplement ouverte reçoit le silence, indéfiniment, et rien ne le signale.
   Deux heures et demie d'écoute perdues le 13/10/2026 pour cette ligne.

2. **La grappe Alimentation est sur le point d'extrémité 1**, avec le robot,
   pas sur le 0 — où l'on conclut à tort que la batterie n'est pas exposée.
"""
from __future__ import annotations

import asyncio
import json
from typing import Awaitable, Callable

import websockets

# Attributs suivis : "endpoint/grappe/attribut"
RUN_MODES = "1/84/0"
RUN_MODE = "1/84/1"
CLEAN_MODES = "1/85/0"
CLEAN_MODE = "1/85/1"
PHASES = "1/97/0"
PHASE = "1/97/1"
OPERATIONAL = "1/97/4"
ERREUR = "1/97/5"
ZONES = "1/336/0"
ZONES_CHOISIES = "1/336/2"
ZONE_COURANTE = "1/336/3"
BATTERIE = "1/47/12"          # en DEMI-pourcents : 200 = 100 %
ETAT_CHARGE = "1/47/26"

SUIVIS = {
    RUN_MODE, CLEAN_MODE, PHASE, OPERATIONAL, ERREUR,
    ZONES, ZONES_CHOISIES, ZONE_COURANTE, BATTERIE, ETAT_CHARGE,
}

CLUSTER_RUN_MODE = 84
CLUSTER_CLEAN_MODE = 85
CLUSTER_OPERATIONAL = 97
CLUSTER_SERVICE_AREA = 336


class MatterIndisponible(Exception):
    pass


class ClientMatter:
    """Une connexion, un cache d'attributs, et des commandes."""

    def __init__(self, url: str, node_id: int, sur_attribut: Callable[[str, object], Awaitable[None]]) -> None:
        self.url = url
        self.node_id = node_id
        self.sur_attribut = sur_attribut
        self.attributs: dict[str, object] = {}
        self._ws = None
        self._compteur = 0
        self._attentes: dict[str, asyncio.Future] = {}
        self._connecte = asyncio.Event()

    @property
    def connecte(self) -> bool:
        return self._connecte.is_set()

    # -- boucle de vie ------------------------------------------------------

    async def tourner(self) -> None:
        """Se connecte, écoute, et se rebranche indéfiniment."""
        while True:
            try:
                async with websockets.connect(self.url, max_size=None) as ws:
                    self._ws = ws
                    await ws.recv()  # bannière du serveur
                    # Le LECTEUR d'abord. Attendre une réponse avant de lire
                    # la socket est un interblocage : personne ne dépile le
                    # message, l'attente expire, et on conclut à tort que le
                    # serveur ne répond pas.
                    lecteur = asyncio.create_task(self._ecouter(ws))
                    try:
                        # L'ABONNEMENT : sans cette commande, aucun événement
                        # n'arrive jamais. Sa réponse contient en prime l'état
                        # complet de tous les nœuds.
                        nodes = await self._demander(ws, "start_listening", {})
                        self._ingerer_nodes(nodes)
                        self._connecte.set()
                        print(f"[robot] connecté à {self.url}, {len(self.attributs)} attributs", flush=True)
                        await lecteur
                    finally:
                        lecteur.cancel()
            except Exception as e:                       # noqa: BLE001
                print(f"[robot] lien Matter rompu ({type(e).__name__}: {e}), nouvelle tentative dans 10 s", flush=True)
            finally:
                self._connecte.clear()
                self._ws = None
                for f in self._attentes.values():
                    if not f.done():
                        f.set_exception(MatterIndisponible("lien Matter rompu"))
                self._attentes.clear()
            await asyncio.sleep(10)

    async def _demander(self, ws, commande: str, args: dict, delai: float = 20.0):
        self._compteur += 1
        mid = f"m{self._compteur}"
        attente: asyncio.Future = asyncio.get_running_loop().create_future()
        self._attentes[mid] = attente
        await ws.send(json.dumps({"message_id": mid, "command": commande, "args": args}))
        try:
            return await asyncio.wait_for(attente, timeout=delai)
        finally:
            self._attentes.pop(mid, None)

    async def _ecouter(self, ws) -> None:
        async for brut in ws:
            m = json.loads(brut)
            mid = m.get("message_id")
            if mid and mid in self._attentes:
                f = self._attentes[mid]
                if not f.done():
                    if "error_code" in m:
                        f.set_exception(MatterIndisponible(str(m.get("details") or m.get("error_code"))))
                    else:
                        f.set_result(m.get("result"))
                continue
            if m.get("event") != "attribute_updated":
                continue
            d = m.get("data") or []
            if len(d) != 3:
                continue
            node_id, chemin, valeur = d
            if int(node_id) != self.node_id or chemin not in SUIVIS:
                continue
            self.attributs[chemin] = valeur
            await self.sur_attribut(chemin, valeur)

    def _ingerer_nodes(self, nodes) -> None:
        for node in nodes or []:
            if int(node.get("node_id", -1)) != self.node_id:
                continue
            for chemin, valeur in (node.get("attributes") or {}).items():
                self.attributs[chemin] = valeur

    # -- commandes ----------------------------------------------------------

    async def commander(self, cluster: int, nom: str, charge: dict | None = None, endpoint: int = 1):
        if self._ws is None:
            raise MatterIndisponible("pas de lien Matter")
        return await self._demander(
            self._ws,
            "device_command",
            {
                "node_id": self.node_id,
                "endpoint_id": endpoint,
                "cluster_id": cluster,
                "command_name": nom,
                "payload": charge or {},
            },
        )

    async def relire(self, chemin: str):
        """Relit un attribut à la source (filet quand une poussée a manqué)."""
        if self._ws is None:
            raise MatterIndisponible("pas de lien Matter")
        valeur = await self._demander(
            self._ws, "read_attribute", {"node_id": self.node_id, "attribute_path": chemin}
        )
        # read_attribute rend parfois un dict {chemin: valeur}
        if isinstance(valeur, dict) and chemin in valeur:
            valeur = valeur[chemin]
        self.attributs[chemin] = valeur
        return valeur
