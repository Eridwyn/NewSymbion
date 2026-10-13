"""Traduction de l'état Matter brut en état Symbion, et diffusion.

Ce fichier contient les deux vérités qu'on a payées à l'usage :

· **Le mode de marche MENT sur la disponibilité.** Il dit `Idle` pendant que
  la station sèche les serpillères pendant quatre heures. C'est la PHASE
  (`1/97/1`) qui dit la vérité : `Dry Mop`.

· **Matter n'expose aucun temps restant.** `CountdownTime` existe dans la
  spécification, Ecovacs ne l'implémente pas. Le séchage se voit, sa durée
  restante s'ESTIME — en horodatant soi-même l'entrée dans la phase. On
  l'étiquette comme une estimation, et on ne la donne pas du tout si on a
  démarré alors que le séchage était déjà en cours.
"""
from __future__ import annotations

import time

from matter_ws import (
    BATTERIE, CLEAN_MODE, CLEAN_MODES, ETAT_CHARGE, OPERATIONAL, ERREUR,
    PHASE, PHASES, RUN_MODE, RUN_MODES, ZONE_COURANTE, ZONES, ZONES_CHOISIES,
)

ETATS_OPERATIONNELS = {
    0: "Arrêté", 1: "En marche", 2: "En pause", 3: "En erreur",
    64: "Cherche la base", 65: "En charge", 66: "À la base",
}
ETATS_CHARGE = {0: "inconnu", 1: "en charge", 2: "pleine", 3: "pas en charge"}

# `1/97/5` (OperationalError) est le SEUL endroit où Matter dit l'état des
# consommables : Ecovacs n'expose pas de grappe « station ». Bac plein,
# réservoir vide, serpillère absente arrivent ici, sous forme d'erreur.
ERREURS = {
    0: "aucune",
    1: "impossible de démarrer ou reprendre",
    2: "impossible de terminer l'opération",
    3: "commande invalide dans cet état",
    64: "base de charge introuvable",
    65: "coincé",
    66: "bac à poussière absent",
    67: "bac à poussière plein",
    68: "réservoir d'eau vide",
    69: "réservoir d'eau absent",
    70: "couvercle du réservoir ouvert",
    71: "serpillère absente",
}
PHASE_SECHAGE = "Dry Mop"
# Les modes de nettoyage qui mobilisent les serpillères : les lancer pendant
# le séchage fait aspirer le robot SUR PLACE, sans qu'il quitte la base.
MODES_AVEC_SERPILLERE = {"Vacuum & Mop", "Mopping", "Mop after Vacuum"}


def _libelle_mode(liste, index):
    """Les listes de modes Matter arrivent en [{"0": libellé, "1": index, …}]."""
    if index is None:
        return None
    for entree in liste or []:
        if entree.get("1") == index:
            return entree.get("0")
    return None


def _zones(brut) -> dict[int, str | None]:
    """Les zones sans nom restent SANS NOM.

    La carte porte des entrées techniques (un repère, pas une pièce) dont
    `locationInfo` est nul. Leur inventer un nom les ferait apparaître comme
    des pièces nettoyables dans le rule builder."""
    sortie: dict[int, str | None] = {}
    for z in brut or []:
        zid = z.get("0")
        if zid is None:
            continue
        info = ((z.get("2") or {}).get("0") or {})
        sortie[zid] = info.get("0") or None
    return sortie


def _erreur(brut) -> dict:
    code = (brut or {}).get("0") if isinstance(brut, dict) else brut
    return {"code": code, "label": ERREURS.get(code, f"code {code}")}


class Etat:
    def __init__(self, client, bus, noyau, pieces: dict[str, str], sechage_nominal: int, seuil_batterie: int) -> None:
        self.client = client
        self.bus = bus
        self.noyau = noyau
        # Table robot → Symbion. Deux écarts à gérer, et aucun des deux n'est
        # un détail :
        #   · les noms diffèrent (« Salle de séjour » chez Ecovacs, « salon »
        #     pour les lampes) ;
        #   · une zone du robot couvre PLUSIEURS pièces au sens des lampes —
        #     la cuisine et le salon sont tous deux dans « Salle de séjour ».
        # D'où une LISTE de pièces par zone. Une valeur simple dans le TOML
        # reste acceptée et vaut une liste d'un élément.
        self.pieces: dict[str, list[str]] = {}
        for zone, cibles in (pieces or {}).items():
            if isinstance(cibles, str):
                cibles = [cibles]
            propres = [str(c).strip().lower() for c in cibles if str(c).strip()]
            if propres:
                self.pieces[zone.strip().lower()] = propres
        self.sechage_nominal = sechage_nominal
        self.seuil_batterie = seuil_batterie
        self._sechage_depuis: float | None = None
        self._precedent: dict = {}

    # -- lecture ------------------------------------------------------------

    def pieces_de_zone(self, nom_zone: str | None) -> list[str]:
        """Toutes les pièces Symbion couvertes par une zone du robot.

        Une zone absente de la table garde son propre nom en minuscules — pas
        une liste vide : sinon une règle ne pourrait jamais viser une pièce
        qu'on a oublié de déclarer, et rien ne le dirait."""
        if not nom_zone:
            return []
        cle = nom_zone.strip().lower()
        return list(self.pieces.get(cle, [cle]))

    def piece_symbion(self, nom_zone: str | None) -> str | None:
        """La pièce principale — pour l'affichage. Les règles visent `rooms`."""
        pieces = self.pieces_de_zone(nom_zone)
        return pieces[0] if pieces else None

    def zones(self) -> dict[int, str]:
        return _zones(self.client.attributs.get(ZONES))

    def vue(self) -> dict:
        a = self.client.attributs
        zones = self.zones()
        run = a.get(RUN_MODE)
        clean = a.get(CLEAN_MODE)
        phase = a.get(PHASE)
        phases = a.get(PHASES) or []
        phase_libelle = phases[phase] if isinstance(phase, int) and phase < len(phases) else None
        oper = a.get(OPERATIONAL)
        batt = a.get(BATTERIE)
        zone_id = a.get(ZONE_COURANTE)
        zone_nom = zones.get(zone_id) if zone_id is not None else None
        run_libelle = _libelle_mode(a.get(RUN_MODES), run)
        clean_libelle = _libelle_mode(a.get(CLEAN_MODES), clean)

        return {
            "online": self.client.connecte,
            "run_mode": {"index": run, "label": run_libelle},
            "clean_mode": {"index": clean, "label": clean_libelle},
            "phase": {"index": phase, "label": phase_libelle},
            "operational": {"code": oper, "label": ETATS_OPERATIONNELS.get(oper, f"code {oper}")},
            "error": _erreur(a.get(ERREUR)),
            "cleaning": run_libelle == "Cleaning",
            "docked": oper in (65, 66),
            "zone": None if zone_id is None else {
                "id": zone_id, "name": zone_nom,
                "room": self.piece_symbion(zone_nom),
                "rooms": self.pieces_de_zone(zone_nom),
            },
            "selected_zones": [
                {"id": z, "name": zones.get(z),
                 "room": self.piece_symbion(zones.get(z)),
                 "rooms": self.pieces_de_zone(zones.get(z))}
                for z in (a.get(ZONES_CHOISIES) or [])
            ],
            "zones": [
                {"id": z, "name": n,
                 "room": self.piece_symbion(n),
                 "rooms": self.pieces_de_zone(n)}
                for z, n in sorted(zones.items())
            ],
            # La batterie arrive en DEMI-pour-cents (200 = 100 %). On arrondit
            # vers le BAS : mieux vaut sous-estimer une charge que la
            # surestimer, et une règle « rentre sous 20 % » doit partir au
            # premier instant où c'est vrai.
            "battery": None if batt is None else int(batt // 2),
            "charge_state": ETATS_CHARGE.get(a.get(ETAT_CHARGE), "inconnu"),
            "drying": self.sechage(phase_libelle),
        }

    def sechage(self, phase_libelle: str | None) -> dict:
        actif = phase_libelle == PHASE_SECHAGE
        if not actif:
            return {"active": False}
        if self._sechage_depuis is None:
            # On a démarré le plugin alors que ça séchait déjà : on ne sait
            # pas depuis quand, et on ne l'invente pas.
            return {"active": True, "elapsed_minutes": None, "estimated_remaining_minutes": None}
        ecoule = round((time.monotonic() - self._sechage_depuis) / 60)
        return {
            "active": True,
            "elapsed_minutes": ecoule,
            "estimated_remaining_minutes": max(0, self.sechage_nominal - ecoule),
            "estimation": True,
        }

    # -- diffusion ----------------------------------------------------------

    async def sur_attribut(self, chemin: str, valeur) -> None:
        await self.diffuser()

    async def diffuser(self) -> None:
        vue = self.vue()
        phase_libelle = (vue["phase"] or {}).get("label")

        # horodatage du séchage : à l'entrée dans la phase, pas avant
        if phase_libelle == PHASE_SECHAGE and self._sechage_depuis is None:
            if self._precedent.get("phase") not in (None, PHASE_SECHAGE):
                self._sechage_depuis = time.monotonic()
        elif phase_libelle != PHASE_SECHAGE:
            self._sechage_depuis = None

        self.bus.publier("symbion/robot/state", vue, retain=True)
        self.bus.publier("symbion/robot/zones", {"zones": vue["zones"]}, retain=True)

        zone = (vue.get("zone") or {})
        features = {
            "robot.run_mode": vue["run_mode"]["label"],
            "robot.phase": phase_libelle,
            "robot.cleaning": vue["cleaning"],
            "robot.docked": vue["docked"],
            "robot.drying": bool(vue["drying"]["active"]),
            "robot.zone": zone.get("name"),
            "robot.room": zone.get("room"),
            # La LISTE des pièces couvertes : c'est la feature qu'une condition
            # interroge (`robot.rooms` contains « salon »). Le kernel en fait
            # une StringList, donc la comparaison est élément par élément —
            # « salon » ne matche pas dans « salon-bis ».
            "robot.rooms": zone.get("rooms") or [],
            "robot.battery": vue["battery"],
            "robot.error": vue["error"]["label"],
            "robot.error_code": vue["error"]["code"],
        }
        for fid, val in features.items():
            if val is not None:
                self.bus.feature(fid, val)

        # Les features d'abord, acquittées par le courtier, PUIS l'événement :
        # une condition de règle lit l'état vivant, et doit le lire à jour.
        self.bus.attendre_remise()

        await self._evenements(vue, phase_libelle, zone)
        self._precedent = {
            "phase": phase_libelle,
            "run_mode": vue["run_mode"]["label"],
            "zone_id": zone.get("id"),
            "battery": vue["battery"],
            "error_code": vue["error"]["code"],
        }

    async def _evenements(self, vue: dict, phase_libelle, zone: dict) -> None:
        """Pousse au kernel ce qui a CHANGÉ — jamais l'état répété.

        Une automation qui se redéclencherait à chaque poussée rallumerait la
        pièce que le porteur vient d'éteindre."""
        avant = self._precedent
        if not avant:
            return  # premier passage : on ne réveille rien sur un état initial

        if zone.get("id") != avant.get("zone_id"):
            await self.noyau.pousser("zone_changed", {
                "zone_id": zone.get("id"),
                "zone": zone.get("name"),
                "room": zone.get("room"),
                "rooms": zone.get("rooms") or [],
                "previous_zone_id": avant.get("zone_id"),
                "cleaning": vue["cleaning"],
            })

        if vue["run_mode"]["label"] != avant.get("run_mode"):
            await self.noyau.pousser("run_mode_changed", {
                "mode": vue["run_mode"]["label"],
                "previous_mode": avant.get("run_mode"),
            })
            if avant.get("run_mode") == "Cleaning" and vue["run_mode"]["label"] != "Cleaning":
                await self.noyau.pousser("cleaning_finished", {
                    "battery": vue["battery"], "last_zone": avant.get("zone_id"),
                })
            if vue["run_mode"]["label"] == "Cleaning":
                await self.noyau.pousser("cleaning_started", {
                    "clean_mode": vue["clean_mode"]["label"],
                    "zones": [z["id"] for z in vue["selected_zones"]],
                })

        if phase_libelle != avant.get("phase"):
            await self.noyau.pousser("phase_changed", {
                "phase": phase_libelle, "previous_phase": avant.get("phase"),
            })
            if phase_libelle == PHASE_SECHAGE:
                await self.noyau.pousser("drying_started", {})
            elif avant.get("phase") == PHASE_SECHAGE:
                await self.noyau.pousser("drying_finished", {})

        if vue["error"]["code"] != avant.get("error_code"):
            # C'est par là que remontent « bac plein » et « réservoir vide » :
            # un événement, pas un sondage.
            await self.noyau.pousser("error_changed", {
                "code": vue["error"]["code"],
                "error": vue["error"]["label"],
                "previous_code": avant.get("error_code"),
            })

        batt, batt_avant = vue["battery"], avant.get("battery")
        if batt is not None and batt_avant is not None:
            if batt <= self.seuil_batterie < batt_avant:
                await self.noyau.pousser("battery_low", {"battery": batt, "threshold": self.seuil_batterie})
