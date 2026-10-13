"""Lien avec le kernel : enregistrement, et poussée d'événements.

La poussée d'événements est la pièce qui manquait : sans elle un plugin ne
peut qu'être APPELÉ par une automation, jamais en réveiller une. Le jeton se
lit dans `data/plugin-event-token`, écrit par le kernel en 0600 — c'est la
permission du fichier qui authentifie, pas une confiance dans l'adresse.
"""
from __future__ import annotations

import asyncio
import ssl
from pathlib import Path

import aiohttp

CHEMIN_JETON = Path("data/plugin-event-token")


class Noyau:
    def __init__(self, url: str, nom_plugin: str, racine: Path | None = None) -> None:
        self.url = url.rstrip("/")
        self.nom = nom_plugin
        self.racine = racine or Path.cwd()
        self._jeton: str | None = None
        self._jeton_deplore = False   # on ne répète pas la plainte à chaque événement
        # Le kernel sert un certificat auto-signé en local : on ne le vérifie
        # pas (c'est ce que fait symbion-plugin-common côté Rust), mais on ne
        # parle qu'à localhost.
        self._ssl = ssl.create_default_context()
        self._ssl.check_hostname = False
        self._ssl.verify_mode = ssl.CERT_NONE

    @property
    def jeton(self) -> str | None:
        if self._jeton is None:
            chemin = self.racine / CHEMIN_JETON
            try:
                self._jeton = chemin.read_text(encoding="utf-8").strip() or None
                self._jeton_deplore = False
            except OSError as e:
                # Le kernel crée ce fichier au premier événement reçu : au
                # démarrage il peut légitimement ne pas exister encore, d'où
                # la relecture à chaque appel — mais une seule plainte.
                if not self._jeton_deplore:
                    print(f"[{self.nom}] jeton d'événements illisible ({chemin}) : {e}", flush=True)
                    self._jeton_deplore = True
                self._jeton = None
        return self._jeton

    async def enregistrer(
        self,
        socket_path: str,
        routes: list[str],
        actions: list[dict],
        version: str,
        description: str,
        essais: int = 30,
    ) -> bool:
        """Insiste : au démarrage de la machine le kernel peut n'être pas prêt."""
        charge = {
            "name": self.nom,
            "socket_path": socket_path,
            "routes": routes,
            "version": version,
            "description": description,
            "actions": actions,
        }
        for essai in range(1, essais + 1):
            try:
                async with aiohttp.ClientSession() as s:
                    async with s.post(
                        f"{self.url}/v1/plugins/register",
                        json=charge,
                        ssl=self._ssl,
                        timeout=aiohttp.ClientTimeout(total=5),
                    ) as r:
                        corps = await r.text()
                        if r.status == 200:
                            print(f"[{self.nom}] enregistré auprès du kernel", flush=True)
                            return True
                        print(f"[{self.nom}] enregistrement refusé ({r.status}) : {corps}", flush=True)
            except Exception as e:                       # noqa: BLE001
                if essai == 1 or essai % 10 == 0:
                    print(f"[{self.nom}] kernel injoignable ({e}), essai {essai}/{essais}", flush=True)
            await asyncio.sleep(2)
        return False

    async def pousser(self, trigger_type: str, donnees: dict) -> None:
        """Réveille les automations dont le Trigger::Custom correspond."""
        jeton = self.jeton
        if not jeton:
            return
        try:
            async with aiohttp.ClientSession() as s:
                async with s.post(
                    f"{self.url}/v1/automations/events",
                    json={
                        "plugin_name": self.nom,
                        "trigger_type": trigger_type,
                        "data": donnees,
                    },
                    headers={"X-Plugin-Token": jeton},
                    ssl=self._ssl,
                    timeout=aiohttp.ClientTimeout(total=5),
                ) as r:
                    if r.status != 200:
                        print(
                            f"[{self.nom}] événement {trigger_type} refusé ({r.status}) : "
                            f"{await r.text()}",
                            flush=True,
                        )
        except Exception as e:                           # noqa: BLE001
            print(f"[{self.nom}] événement {trigger_type} non poussé : {e}", flush=True)
