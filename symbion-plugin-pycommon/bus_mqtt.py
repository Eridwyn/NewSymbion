"""Publication MQTT — le contrat que le reste de Symbion lit.

Partagé par les plugins Python (lights, robot), pendant de
`symbion-plugin-common` côté Rust. Chaque plugin publie :
  · `symbion/{plugin}/…`        son état métier (retenu)
  · `symbion/features/update`   les features que les automations mettent en condition
  · `symbion/plugins/{plugin}/` son manifeste et sa santé

L'état est RETENU : un tableau de bord qui se connecte doit voir l'état tout
de suite, sans attendre le prochain changement. Le testament MQTT (`will`)
marque le plugin hors ligne si le processus meurt sans prévenir — c'est ce qui
permet au kernel de voir une panne au lieu d'un dernier état figé.
"""
from __future__ import annotations

import json
import threading

import paho.mqtt.client as mqtt

class BusMqtt:
    def __init__(self, hote: str, port: int, plugin: str) -> None:
        self.hote = hote
        self.port = port
        self.plugin = plugin
        self._client = mqtt.Client(client_id=f"symbion-plugin-{plugin}", clean_session=True)
        self._client.will_set(
            f"symbion/plugins/{plugin}/status",
            json.dumps({"plugin": plugin, "status": "offline"}),
            qos=1,
            retain=True,
        )
        self._pret = threading.Event()
        self._en_vol: list = []
        self._client.on_connect = self._sur_connexion

    def _sur_connexion(self, client, userdata, flags, rc) -> None:  # noqa: ANN001
        if rc == 0:
            self._pret.set()
            print(f"[{self.plugin}] MQTT connecté à {self.hote}:{self.port}", flush=True)
        else:
            print(f"[{self.plugin}] MQTT refus de connexion (rc={rc})", flush=True)

    def demarrer(self) -> None:
        self._client.connect_async(self.hote, self.port, keepalive=30)
        self._client.loop_start()

    def arreter(self) -> None:
        try:
            self.publier(
                f"symbion/plugins/{self.plugin}/status",
                {"plugin": self.plugin, "status": "offline"},
                retain=True,
            )
            self._client.loop_stop()
            self._client.disconnect()
        except Exception:                                # noqa: BLE001
            pass

    def publier(self, sujet: str, charge: dict, retain: bool = False) -> None:
        try:
            info = self._client.publish(sujet, json.dumps(charge, ensure_ascii=False), qos=1, retain=retain)
            self._en_vol.append(info)
        except Exception as e:                           # noqa: BLE001
            print(f"[{self.plugin}] MQTT publication échouée sur {sujet} : {e}", flush=True)

    def attendre_remise(self, delai: float = 2.0) -> None:
        """Attend l'acquittement du courtier pour tout ce qui est en vol.

        Sert à ORDONNER : une automation déclenchée par un événement HTTP peut
        mettre une feature en condition, et cette feature voyage par MQTT. Si
        l'événement arrive au kernel avant la feature, la condition est
        évaluée sur l'état PRÉCÉDENT — la lampe de la pièce qu'il vient de
        quitter. On vide donc la file avant de pousser l'événement.
        """
        en_vol, self._en_vol = self._en_vol, []
        for info in en_vol:
            try:
                info.wait_for_publish(timeout=delai)
            except Exception:                            # noqa: BLE001
                pass

    def feature(self, feature_id: str, valeur, ttl_seconds: int = 3600) -> None:
        """Une feature = ce qu'une automation peut mettre en condition."""
        self.publier(
            "symbion/features/update",
            {
                "feature_id": feature_id,
                "value": valeur,
                "source": self.plugin,
                "ttl_seconds": ttl_seconds,
            },
        )
