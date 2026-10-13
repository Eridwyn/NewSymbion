#!/usr/bin/env python3
"""Essais du plugin robot — sans robot, sans serveur Matter, sans kernel.

    ./.venv/bin/python essais.py

On éprouve la traduction des attributs bruts (c'est là que les erreurs se
cachent : un libellé pris pour un index, une zone technique prise pour une
pièce) et la règle « un événement par changement, pas par poussée ».
"""
from __future__ import annotations

import asyncio
import sys
from pathlib import Path

_ici = Path(__file__).resolve().parent
sys.path.insert(0, str(_ici / "src"))
sys.path.insert(0, str(_ici.parent / "symbion-plugin-pycommon"))

import matter_ws as M
from etat import Etat, _erreur, _libelle_mode, _zones

essais = []


def essai(f):
    essais.append(f)
    return f


class BusMuet:
    def __init__(self):
        self.features = []

    def publier(self, sujet, charge, retain=False):
        pass

    def attendre_remise(self, delai=2.0):
        pass

    def feature(self, fid, valeur, ttl_seconds=3600):
        self.features.append((fid, valeur))


class NoyauMuet:
    def __init__(self):
        self.pousses = []

    async def pousser(self, type_, donnees):
        self.pousses.append((type_, donnees))


class ClientMuet:
    def __init__(self, attributs):
        self.attributs = dict(attributs)
        self.connecte = True


ATTRIBUTS = {
    M.RUN_MODES: [{"0": "Idle", "1": 0}, {"0": "Cleaning", "1": 1}, {"0": "Mapping", "1": 2}],
    M.RUN_MODE: 0,
    M.CLEAN_MODES: [{"0": "Vacuum & Mop", "1": 0}, {"0": "Vacuum-Only", "1": 1}],
    M.CLEAN_MODE: 0,
    M.PHASES: ["Return", "Goto", "Idle", "Relocation", "Dry Mop", "General Clean"],
    M.PHASE: 2,
    M.OPERATIONAL: 66,
    M.ERREUR: {"0": 0},
    M.ZONES: [
        {"0": 10000, "2": {"0": {"0": "Couloir"}}},
        {"0": 10003, "2": {"0": {"0": "Salle de séjour"}}},
        {"0": 11002, "2": {"0": None, "1": {"0": 11}}},
    ],
    M.ZONES_CHOISIES: [10000],
    M.ZONE_COURANTE: None,
    M.BATTERIE: 200,
    M.ETAT_CHARGE: 2,
}

PIECES = {"Salle de séjour": ["salon", "cuisine"], "Couloir": "couloir"}


def etat_d_essai(**remplacements):
    attrs = dict(ATTRIBUTS)
    attrs.update(remplacements)
    client = ClientMuet(attrs)
    bus, noyau = BusMuet(), NoyauMuet()
    return Etat(client, bus, noyau, PIECES, sechage_nominal=240, seuil_batterie=20), bus, noyau


# -- traduction ------------------------------------------------------------

@essai
def libelle_de_mode_par_index():
    assert _libelle_mode(ATTRIBUTS[M.RUN_MODES], 1) == "Cleaning"
    assert _libelle_mode(ATTRIBUTS[M.RUN_MODES], 9) is None
    assert _libelle_mode(ATTRIBUTS[M.RUN_MODES], None) is None


@essai
def zone_sans_nom_reste_sans_nom():
    z = _zones(ATTRIBUTS[M.ZONES])
    assert z[10000] == "Couloir"
    assert z[11002] is None, "une zone technique a reçu un nom inventé"


@essai
def erreur_decodee_en_clair():
    assert _erreur({"0": 0})["label"] == "aucune"
    assert _erreur({"0": 67})["label"] == "bac à poussière plein"
    assert _erreur({"0": 68})["label"] == "réservoir d'eau vide"
    assert "code 999" in _erreur({"0": 999})["label"]


@essai
def pieces_traduites_vers_symbion():
    e, _, _ = etat_d_essai()
    assert e.piece_symbion("Salle de séjour") == "salon"
    # une pièce absente de la table garde son nom, en minuscules — pas None,
    # sinon une règle ne pourrait jamais la viser
    assert e.piece_symbion("Chambre") == "chambre"
    assert e.piece_symbion(None) is None


@essai
def une_zone_couvre_plusieurs_pieces():
    # Le cas réel : cuisine et salon sont tous deux dans « Salle de séjour ».
    e, _, _ = etat_d_essai()
    assert e.pieces_de_zone("Salle de séjour") == ["salon", "cuisine"]
    # valeur simple dans le TOML = liste d'un élément
    assert e.pieces_de_zone("Couloir") == ["couloir"]
    # zone non déclarée : son propre nom, pas une liste vide
    assert e.pieces_de_zone("Chambre") == ["chambre"]
    assert e.pieces_de_zone(None) == []


@essai
def batterie_en_demi_pourcents():
    e, _, _ = etat_d_essai()
    assert e.vue()["battery"] == 100
    # 37 demi-pour-cents = 18,5 % → arrondi vers le BAS, on ne surestime
    # jamais une batterie
    e2, _, _ = etat_d_essai(**{M.BATTERIE: 37})
    assert e2.vue()["battery"] == 18


@essai
def a_la_base_et_pas_en_nettoyage():
    v = etat_d_essai()[0].vue()
    assert v["docked"] is True and v["cleaning"] is False
    assert v["operational"]["label"] == "À la base"


@essai
def le_mode_de_marche_mente_pendant_le_sechage():
    # run mode = Idle ALORS QUE la station sèche : c'est la phase qui dit vrai
    e, _, _ = etat_d_essai(**{M.PHASE: 4})
    v = e.vue()
    assert v["run_mode"]["label"] == "Idle"
    assert v["phase"]["label"] == "Dry Mop"
    assert v["drying"]["active"] is True


@essai
def sechage_demarre_avant_nous_ne_sinvente_pas():
    e, _, _ = etat_d_essai(**{M.PHASE: 4})
    s = e.sechage("Dry Mop")
    assert s["active"] is True
    assert s["elapsed_minutes"] is None, "une durée a été inventée"
    assert s["estimated_remaining_minutes"] is None


# -- événements ------------------------------------------------------------

@essai
def premier_passage_ne_reveille_rien():
    async def scenario():
        e, _, noyau = etat_d_essai()
        await e.diffuser()
        assert noyau.pousses == [], noyau.pousses
    asyncio.run(scenario())


@essai
def changement_de_zone_pousse_la_piece_symbion():
    async def scenario():
        e, _, noyau = etat_d_essai()
        await e.diffuser()                      # état initial
        e.client.attributs[M.ZONE_COURANTE] = 10003
        e.client.attributs[M.RUN_MODE] = 1
        await e.diffuser()
        zones = [d for t, d in noyau.pousses if t == "zone_changed"]
        assert len(zones) == 1, noyau.pousses
        assert zones[0]["zone"] == "Salle de séjour"
        assert zones[0]["room"] == "salon", zones[0]
        # les DEUX pièces partent : c'est sur `rooms` qu'une règle se vise
        assert zones[0]["rooms"] == ["salon", "cuisine"], zones[0]
        assert zones[0]["cleaning"] is True
        # rediffuser le même état ne doit RIEN repousser
        avant = len(noyau.pousses)
        await e.diffuser()
        assert len(noyau.pousses) == avant, "un état identique a été repoussé"
    asyncio.run(scenario())


@essai
def fin_de_nettoyage_signalee():
    async def scenario():
        e, _, noyau = etat_d_essai(**{M.RUN_MODE: 1})
        await e.diffuser()
        e.client.attributs[M.RUN_MODE] = 0
        await e.diffuser()
        types = [t for t, _ in noyau.pousses]
        assert "cleaning_finished" in types, types
    asyncio.run(scenario())


@essai
def bac_plein_remonte_comme_evenement():
    async def scenario():
        e, _, noyau = etat_d_essai()
        await e.diffuser()
        e.client.attributs[M.ERREUR] = {"0": 67}
        await e.diffuser()
        erreurs = [d for t, d in noyau.pousses if t == "error_changed"]
        assert erreurs and erreurs[0]["error"] == "bac à poussière plein", noyau.pousses
    asyncio.run(scenario())


@essai
def batterie_basse_une_seule_fois():
    async def scenario():
        e, _, noyau = etat_d_essai(**{M.BATTERIE: 60})   # 30 %
        await e.diffuser()
        e.client.attributs[M.BATTERIE] = 36              # 18 % → franchit 20
        await e.diffuser()
        e.client.attributs[M.BATTERIE] = 30              # 15 % → déjà franchi
        await e.diffuser()
        bas = [d for t, d in noyau.pousses if t == "battery_low"]
        assert len(bas) == 1, noyau.pousses
    asyncio.run(scenario())


@essai
def features_publiees_pour_les_conditions():
    async def scenario():
        e, bus, _ = etat_d_essai()
        await e.diffuser()
        noms = dict(bus.features)
        for attendu in ("robot.run_mode", "robot.phase", "robot.battery", "robot.docked", "robot.error"):
            assert attendu in noms, noms
        assert noms["robot.battery"] == 100
        # hors nettoyage il ne couvre aucune pièce : liste VIDE, pas absente —
        # une condition « contient salon » doit pouvoir échouer franchement
        assert noms["robot.rooms"] == [], noms["robot.rooms"]


@essai
def feature_rooms_porte_les_deux_pieces():
    async def scenario():
        e, bus, _ = etat_d_essai()
        await e.diffuser()
        e.client.attributs[M.ZONE_COURANTE] = 10003
        await e.diffuser()
        assert dict(bus.features)["robot.rooms"] == ["salon", "cuisine"]
    asyncio.run(scenario())
    asyncio.run(scenario())


if __name__ == "__main__":
    rates = 0
    for f in essais:
        try:
            f()
            print(f"  ok   {f.__name__}")
        except Exception as e:                           # noqa: BLE001
            rates += 1
            print(f"  RATÉ {f.__name__} : {type(e).__name__}: {e}")
    print(f"\n{len(essais) - rates}/{len(essais)} essais passent")
    raise SystemExit(1 if rates else 0)
