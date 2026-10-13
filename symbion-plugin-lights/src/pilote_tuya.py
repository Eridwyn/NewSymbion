"""Pilote Tuya local — un fil d'exécution par lampe, propriétaire de sa prise.

Pourquoi un fil par lampe : une ampoule Tuya n'accepte **qu'une seule
connexion locale à la fois**, et tinytuya est synchrone. Le fil garde la prise
ouverte, applique les commandes qu'on lui dépose et relaie ce que l'ampoule
pousse. Personne d'autre ne touche à la prise — c'est ce qui évite les
déconnexions croisées quand l'API et l'écoute tirent dessus en même temps.

Le pilote ne renvoie jamais d'unités Tuya : il traduit en pourcents et en
Kelvin avant de sortir (cf. modele.py).
"""
from __future__ import annotations

import queue
import threading
import time
from typing import Callable

import tinytuya

from modele import Commande, borne_pct, kelvin_vers_pct, pct_vers_kelvin

# Combien de temps on attend une poussée de l'ampoule avant de rendre la main
# au traitement des commandes. Court = commandes réactives, long = moins de
# réveils inutiles. 2 s est le compromis qui tient les deux.
ATTENTE_RECEPTION = 2
INTERVALLE_BATTEMENT = 9      # sous les 10 s au-delà desquelles Tuya coupe
INTERVALLE_RELECTURE = 60     # filet si une poussée a été manquée
ATTENTE_RECONNEXION = 10


class PiloteTuya(threading.Thread):
    """Fil propriétaire d'une ampoule."""

    def __init__(self, conf: dict, publier: Callable[[str, dict], None]) -> None:
        super().__init__(name=f"tuya-{conf['id']}", daemon=True)
        self.conf = conf
        self.id = conf["id"]
        self.publier = publier          # appelé DEPUIS CE FIL : doit être sûr
        self.commandes: queue.Queue[Commande] = queue.Queue()
        self._arret = threading.Event()
        self._ampoule: tinytuya.BulbDevice | None = None

    # -- API appelée depuis la boucle asyncio -------------------------------

    def envoyer(self, commande: Commande) -> None:
        self.commandes.put(commande)

    def arreter(self) -> None:
        self._arret.set()

    # -- vie du fil ---------------------------------------------------------

    def run(self) -> None:
        while not self._arret.is_set():
            try:
                self._brancher()
                self._boucle()
            except Exception as e:                      # noqa: BLE001
                self._hors_ligne(f"{type(e).__name__}: {e}")
                self._debrancher()
                self._arret.wait(ATTENTE_RECONNEXION)
        self._debrancher()

    def _brancher(self) -> None:
        a = tinytuya.BulbDevice(
            self.conf["device_id"],
            self.conf["ip"],
            self.conf["local_key"],
            version=float(self.conf.get("version", 3.3)),
        )
        a.set_socketPersistent(True)
        a.set_socketTimeout(ATTENTE_RECEPTION)
        a.set_socketRetryLimit(1)
        # detect_bulb lit l'ampoule pour savoir quels points de données elle
        # utilise (type A : 1/2/3/4 sur 0-255 · type B : 20/21/22/23 sur
        # 0-1000 · type C : sans mode). C'est ce qui nous dispense de coder
        # une table de DP par modèle — et cette table était fausse pour le
        # type A dans le document de conception.
        a.detect_bulb()
        self._ampoule = a
        self._lire(force=True)

    def _debrancher(self) -> None:
        if self._ampoule is not None:
            try:
                self._ampoule.close()
            except Exception:                            # noqa: BLE001
                pass
            self._ampoule = None

    def _boucle(self) -> None:
        dernier_battement = time.monotonic()
        derniere_relecture = time.monotonic()
        while not self._arret.is_set():
            # 1. les commandes d'abord : c'est ce que l'utilisateur attend
            applique = False
            while True:
                try:
                    cmd = self.commandes.get_nowait()
                except queue.Empty:
                    break
                self._appliquer(cmd)
                applique = True
            if applique:
                # Boucle fermée : on relit l'ampoule au lieu de supposer que
                # la commande a pris. Une lampe qui refuse une valeur ne doit
                # pas apparaître comme l'ayant acceptée.
                self._lire(force=True)
                derniere_relecture = time.monotonic()

            # 2. ce que l'ampoule pousse d'elle-même (Alexa, interrupteur…)
            donnees = None
            try:
                donnees = self._ampoule.receive()
            except Exception:                            # noqa: BLE001
                donnees = None
            if isinstance(donnees, dict) and "dps" in donnees:
                self._depuis_dps(donnees["dps"])

            maintenant = time.monotonic()
            if maintenant - dernier_battement >= INTERVALLE_BATTEMENT:
                self._ampoule.heartbeat(nowait=True)
                dernier_battement = maintenant
            if maintenant - derniere_relecture >= INTERVALLE_RELECTURE:
                self._lire(force=True)
                derniere_relecture = maintenant

    # -- lecture / écriture -------------------------------------------------

    def _appliquer(self, cmd: Commande) -> None:
        a = self._ampoule
        if a is None or cmd.vide():
            return
        # L'ordre compte : on règle avant d'allumer, sinon on voit la lampe
        # sauter à son ancienne valeur une fraction de seconde.
        if cmd.color_temp_k is not None:
            a.set_colourtemp_percentage(kelvin_vers_pct(cmd.color_temp_k), nowait=False)
        if cmd.brightness is not None:
            pct = borne_pct(cmd.brightness)
            if pct is not None:
                a.set_brightness_percentage(pct, nowait=False)
        if cmd.on is not None:
            a.turn_on() if cmd.on else a.turn_off()
        elif cmd.brightness is not None or cmd.color_temp_k is not None:
            # Régler une lampe éteinte sans l'allumer n'a pas de sens pour le
            # porteur : « mets le salon à 30 % » veut dire allumée à 30 %.
            a.turn_on()

    def _lire(self, force: bool = False) -> None:
        a = self._ampoule
        if a is None:
            return
        etat = a.status()
        if not isinstance(etat, dict):
            raise RuntimeError(f"réponse inattendue : {etat!r}")
        if "Error" in etat:
            raise RuntimeError(str(etat.get("Error")))
        self._depuis_dps(etat.get("dps") or {})

    def _depuis_dps(self, dps: dict) -> None:
        """Traduit les points de données bruts en état Symbion."""
        a = self._ampoule
        if a is None:
            return
        jeu = a.dpset or {}
        echelle = jeu.get("value_max") or 0
        maj: dict = {"online": True, "last_error": None}

        def brut(cle: str):
            dp = jeu.get(cle)
            return dps.get(str(dp)) if dp is not None else None

        allume = brut("switch")
        if allume is not None:
            maj["on"] = bool(allume)

        lum = brut("brightness")
        if lum is not None and echelle:
            maj["brightness"] = borne_pct(float(lum) / echelle * 100.0)

        temp = brut("colourtemp")
        if temp is not None and echelle:
            maj["color_temp_k"] = pct_vers_kelvin(float(temp) / echelle * 100.0)

        self.publier(self.id, maj)

    def _hors_ligne(self, raison: str) -> None:
        self.publier(self.id, {"online": False, "last_error": raison})
