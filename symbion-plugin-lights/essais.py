#!/usr/bin/env python3
"""Essais du plugin lumières — sans matériel, sans kernel, sans MQTT.

    ./.venv/bin/python essais.py

On éprouve ce qui peut mentir en silence : la normalisation des unités, et la
règle « ne rien publier si rien n'a changé » (sans quoi une automation se
redéclenche à chaque poussée de l'ampoule et rallume ce que le porteur vient
d'éteindre).
"""
from __future__ import annotations

import asyncio
import json
import sys
import tempfile
from pathlib import Path

_ici = Path(__file__).resolve().parent
sys.path.insert(0, str(_ici / "src"))
sys.path.insert(0, str(_ici.parent / "symbion-plugin-pycommon"))

import config as config_mod
from modele import Commande, Scene, borne_pct, kelvin_vers_pct, pct_vers_kelvin
from registre import Registre

essais = []


def essai(f):
    essais.append(f)
    return f


class BusMuet:
    def __init__(self):
        self.messages = []
        self.features = []

    def publier(self, sujet, charge, retain=False):
        self.messages.append((sujet, charge))

    def attendre_remise(self, delai=2.0):
        pass

    def feature(self, fid, valeur, ttl_seconds=3600):
        self.features.append((fid, valeur))


class NoyauMuet:
    def __init__(self):
        self.pousses = []

    async def pousser(self, type_, donnees):
        self.pousses.append((type_, donnees))


def registre_d_essai(tmp: Path):
    bus, noyau = BusMuet(), NoyauMuet()
    r = Registre(bus, noyau, tmp / "scenes.json")
    r.declarer({"id": "salon_a", "name": "A", "room": "Salon"})
    r.declarer({"id": "salon_b", "name": "B", "room": "salon"})
    r.declarer({"id": "cuisine_a", "name": "C", "room": "cuisine"})
    return r, bus, noyau


# -- normalisation ---------------------------------------------------------

@essai
def kelvin_aller_retour():
    for k in (2200, 2700, 4000, 6500):
        assert abs(pct_vers_kelvin(kelvin_vers_pct(k)) - k) <= 1, k
    # hors bornes : on borne, on ne plante pas
    assert pct_vers_kelvin(kelvin_vers_pct(1000)) == 2200
    assert pct_vers_kelvin(kelvin_vers_pct(9000)) == 6500


@essai
def luminosite_zero_devient_un():
    # Tuya n'accepte pas 0 : une lampe à 0 % est une lampe éteinte, et c'est
    # `on=false` qui le dit — jamais la luminosité.
    assert borne_pct(0) == 1
    assert borne_pct(-5) == 1
    assert borne_pct(250) == 100
    assert borne_pct(None) is None
    assert borne_pct("pouet") is None


@essai
def commande_depuis_json():
    c = Commande.depuis_json({"on": "oui", "brightness": "40"})
    assert c.on is True and c.brightness == 40 and c.color_temp_k is None
    assert Commande.depuis_json({}).vide()
    assert Commande.depuis_json({"on": False}).on is False


# -- registre --------------------------------------------------------------

@essai
def piece_insensible_a_la_casse():
    with tempfile.TemporaryDirectory() as d:
        r, _, _ = registre_d_essai(Path(d))
        assert r.lampes["salon_a"].room == "salon"
        assert sorted(r.pieces()) == ["cuisine", "salon"]


@essai
def rien_ne_part_si_rien_ne_change():
    async def scenario():
        with tempfile.TemporaryDirectory() as d:
            r, bus, noyau = registre_d_essai(Path(d))
            r.armer(asyncio.get_running_loop())
            r._appliquer("salon_a", {"online": True, "on": True, "brightness": 50})
            await asyncio.sleep(0)
            assert len(noyau.pousses) >= 1, noyau.pousses
            avant = len(noyau.pousses)
            # exactement le même état poussé une seconde fois
            r._appliquer("salon_a", {"online": True, "on": True, "brightness": 50})
            await asyncio.sleep(0)
            assert len(noyau.pousses) == avant, "un état identique a été rediffusé"
    asyncio.run(scenario())


@essai
def changement_de_piece_signale_une_seule_fois():
    async def scenario():
        with tempfile.TemporaryDirectory() as d:
            r, bus, noyau = registre_d_essai(Path(d))
            r.armer(asyncio.get_running_loop())
            r._appliquer("salon_a", {"online": True, "on": True})
            await asyncio.sleep(0)
            types = [t for t, _ in noyau.pousses]
            assert types.count("room_any_on_changed") == 1, types
            # la deuxième lampe du salon s'allume : la PIÈCE était déjà allumée
            r._appliquer("salon_b", {"online": True, "on": True})
            await asyncio.sleep(0)
            types = [t for t, _ in noyau.pousses]
            assert types.count("room_any_on_changed") == 1, types
            # on éteint la première : la pièce reste allumée par la seconde
            r._appliquer("salon_a", {"online": True, "on": False})
            await asyncio.sleep(0)
            types = [t for t, _ in noyau.pousses]
            assert types.count("room_any_on_changed") == 1, types
            # la seconde s'éteint : LÀ la pièce s'éteint
            r._appliquer("salon_b", {"online": True, "on": False})
            await asyncio.sleep(0)
            types = [t for t, _ in noyau.pousses]
            assert types.count("room_any_on_changed") == 2, types
    asyncio.run(scenario())


@essai
def feature_globale_pour_ne_pas_tourner_a_vide():
    async def scenario():
        with tempfile.TemporaryDirectory() as d:
            r, bus, _ = registre_d_essai(Path(d))
            r.armer(asyncio.get_running_loop())
            r._appliquer("salon_a", {"online": True, "on": False})
            await asyncio.sleep(0)
            assert dict(bus.features)["lights.any_on"] is False
            assert dict(bus.features)["lights.rooms_on"] == []
            r._appliquer("cuisine_a", {"online": True, "on": True})
            await asyncio.sleep(0)
            assert dict(bus.features)["lights.any_on"] is True
            assert dict(bus.features)["lights.rooms_on"] == ["cuisine"]
            # une lampe injoignable allumée ne compte pas : on ne peut rien
            # éteindre qu'on ne joint pas
            r._appliquer("cuisine_a", {"online": False})
            await asyncio.sleep(0)
            assert dict(bus.features)["lights.any_on"] is False
    asyncio.run(scenario())


@essai
def etat_de_depart_publie_avant_tout_changement():
    with tempfile.TemporaryDirectory() as d:
        r, bus, _ = registre_d_essai(Path(d))
        r.publier_tout()
        noms = dict(bus.features)
        # Une Condition::Feature sur une feature absente est FAUSSE : il faut
        # que tout existe dès le démarrage.
        for attendu in ("lights.any_on", "lights.rooms_on", "lights.salon.any_on",
                        "lights.cuisine.any_on", "lights.salon_a.on"):
            assert attendu in noms, noms


@essai
def scenes_survivent_au_redemarrage():
    with tempfile.TemporaryDirectory() as d:
        chemin = Path(d) / "scenes.json"
        r, _, _ = registre_d_essai(Path(d))
        r.enregistrer_scene(Scene(id="nuit", name="Nuit", targets=[{"light_id": "salon_a", "on": True, "brightness": 10}]))
        assert chemin.exists()
        r2, _, _ = registre_d_essai(Path(d))
        assert "nuit" in r2.scenes and r2.scenes["nuit"].targets[0]["brightness"] == 10


@essai
def config_refuse_une_lampe_incomplete():
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "lights.toml"
        p.write_text('[[lights]]\nid = "a"\ndevice_id = "x"\nip = "1.2.3.4"\n', encoding="utf-8")
        try:
            config_mod.charger(p)
        except config_mod.ConfigInvalide as e:
            assert "local_key" in str(e), e
        else:
            raise AssertionError("une lampe sans clé locale a été acceptée")


@essai
def config_refuse_les_doublons():
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "lights.toml"
        bloc = 'id = "a"\ndevice_id = "x"\nlocal_key = "k"\nip = "1.2.3.4"\n'
        p.write_text(f"[[lights]]\n{bloc}\n[[lights]]\n{bloc}", encoding="utf-8")
        try:
            config_mod.charger(p)
        except config_mod.ConfigInvalide as e:
            assert "double" in str(e), e
        else:
            raise AssertionError("deux lampes de même identifiant ont été acceptées")


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
