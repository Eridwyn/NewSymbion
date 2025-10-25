# 🌌 NewSymbion - Compagnon d'Automatisation Personnelle v1.1.0

**Une extension de ton système nerveux** - Architecture IoT/domotique pour te libérer de la charge mentale du quotidien.

> 🧠 **Vision** : Symbion n'est pas une app, ni un gadget : c'est un **cortex digital** qui veille et agit pour toi, un compagnon intelligent qui te libère pour consacrer ton énergie à ce qui compte vraiment.
>
> 🏡 **Domaines** : Maison, Finance, Santé, Pro, Famille - Une solution complète pour ton écosystème personnel
>
> ⚡ **Modes adaptatifs** : Symbion Cravate 👔 (pro), Symbion Intime 🏡 (maison), Symbion Neutre 🌱 (toujours actif)

---

## 🌟 Exemple : Ta journée avec Symbion

### ☀️ Matin
- **Réveil intelligent** : Symbion détecte que tu te lèves et propose un petit-déjeuner équilibré avec ce qu'il reste dans le frigo
- **Préparation pro** : Planning du jour, notes client préparées automatiquement, rappels contextuels

### 🍽️ Midi  
- **Adaptation mobile** : En déplacement, Symbion suggère des choix repas sains autour de toi selon tes préférences
- **Assistance technique** : Problème client ? Symbion te glisse la commande adaptée ou le diagnostic probable

### 🌙 Soir
- **Retour maison** : La tablette cuisine propose un repas avec les restes, Symbion enregistre ton humeur
- **Préparation nuit** : Sauvegarde silencieuse notes/journal/finances, demain est prêt avant ton réveil

---

## 🏗️ Architecture IoT Distribuée

### 🧬 Composants Intelligents
- **symbion-kernel** : Cerveau central - Event Bus MQTT + IA contextuelle + Plugin orchestration
- **symbion-agent-host** : Agents domestiques - Capteurs maison + contrôle appareils + télémétrie environnementale  
- **symbion-plugin-notes** : Mémoire externe - Journal contextuel + rappels intelligents + apprentissage habitudes
- **pwa-dashboard** : Interface adaptative - Widgets personnalisés + contrôle vocal + notifications proactives

### 🤖 Capacités Domotiques Actuelles
- **🏠 Contrôle Maison** : Extinction/réveil machines, monitoring consommation, détection présence
- **📱 Interface Adaptative** : Dashboard PWA qui s'adapte au contexte (matin = planning, soir = détente)

---

## 📚 Modules de Vie Intégrés (Roadmap)

### 🏡 **Maison Intelligente** 
- **Cuisine connectée** : Gestion frigo + suggestions repas avec restes + listes courses automatiques
- **Ambiance adaptive** : Éclairage/température selon humeur + météo + présence
- **Maintenance préventive** : Rappels entretien équipements + détection pannes

### 💸 **Finance Personnelle**
- **Budget intelligent** : Catégorisation automatique + alertes dépassement + optimisations
- **Épargne contexuelle** : Virements automatiques selon revenus + objectifs personnalisés
- **Analyses prédictives** : Tendances dépenses + conseil investissements

### 💪 **Santé & Bien-être**
- **Routine adaptive** : Exercices selon forme du jour + météo + planning
- **Nutrition optimisée** : Menus selon objectifs santé + contraintes + goûts appris
- **Sommeil intelligent** : Analyse cycles + optimisation environnement chambre

### 👔 **Assistant Professionnel**
- **Gestion clients** : Historique interactions + rappels follow-up + templates personnalisés
- **Productivité contexuelle** : Focus mode selon tâches + interruptions minimisées
- **Veille technologique** : Curation contenu pertinent + apprentissage automatique

### 🤝 **Harmonie Familiale**
- **Coordination activités** : Planning partagé + négociation tâches + mood board
- **Listes collaboratives** : Courses + tâches + projets avec notifications intelligentes
- **Communication facilitée** : Suggestions cadeaux + rappels anniversaires + médiation conflits

---

## 🚀 État Actuel - Foundation Solide

### ✅ **Infrastructure IoT Opérationnelle**
- **Event Bus MQTT** : Communication temps réel entre tous les composants domestiques
- **Agents Multi-OS** : 2 agents actifs (Windows + Linux) avec découverte automatique réseau
- **Plugin System** : Architecture modulaire pour extension fonctionnalités
- **PWA Dashboard** : Interface responsive avec widgets adaptatifs temps réel
- **Contract Registry** : Système de validation événements pour fiabilité IoT

### 🔄 **Fonctions Domotiques Actives**
- **Contrôle système à distance** : Extinction/redémarrage machines + monitoring consommation
- **Télémétrie environnementale** : CPU, RAM, température, processus système en temps réel  
- **Notes contextuelles** : Journal intelligent avec tags automatiques selon SSID/heure
- **Découverte réseau** : Auto-détection appareils domestiques avec priorité Ethernet

---

## 🎛️ Configuration Domotique

### 🏠 **Agent Maison Linux (PC-Salon)**
```toml
# ~/.config/symbion-agent/config.toml
[iot]
discovery_mode = "ethernet_priority"    # Réseau domestique stable
presence_detection = true               # Détection présence via activité
environmental_monitoring = true         # Température, humidité si capteurs

[automation]
context_learning = true                 # Apprentissage habitudes
smart_scheduling = true                 # Tâches selon contexte
energy_optimization = true              # Économies automatiques
```

### 💻 **Agent Bureau Windows (DESKTOP)**  
```toml
[work_mode]
productivity_focus = true               # Mode concentration
meeting_detection = true                # Calendrier intégré
notification_filtering = "work_hours"   # 9h-18h seulement

[health_monitoring]
break_reminders = true                  # Pauses régulières
posture_alerts = true                   # Ergonomie travail
screen_time_tracking = true             # Temps écran
```

---

## ⚡ Modes Contextuels Intelligents (ROADMAP)

### 👔 **Symbion Cravate (Mode Pro)**
- **Détection automatique** : SSID bureau + horaires 9h-18h + processus professionnels actifs
- **Fonctions** : Gestion clients, rappels suivis, notes meetings, concentration focus
- **Interface** : Widgets productivité, calendrier intégré, silencieux personnel

### 🏡 **Symbion Intime (Mode Maison)**
- **Détection** : SSID domicile + soirée + weekend + applications loisir  
- **Fonctions** : Cuisine connectée, ambiance adaptive, entertainment, famille
- **Interface** : Widgets confort, suggestions détente, contrôles domotique

### 🌱 **Symbion Neutre (Mode Base)**
- **Toujours actif** : Surveillance système, apprentissage patterns, maintenance
- **Fonctions** : Santé machines, sauvegardes, mises à jour, métriques
- **Interface** : Widgets système, monitoring, notifications critiques

---

## 🛠️ Installation Écosystème Domestique

### 🧬 **1. Kernel Central (Cerveau de la maison)**
```bash
git clone https://github.com/Eridwyn/NewSymbion
cd NewSymbion/symbion-kernel

# Démarrage cerveau central
SYMBION_API_KEY="your-secure-key" cargo run

# ✅ Résultat : Hub domestique actif
# [kernel] IoT Hub listening on :8080  
# [agents] 0 domestic agents registered
# [plugins] notes-manager active (mémoire externe)
```

### 🤖 **2. Agents Domestiques (Un par pièce/appareil)**
```bash
# Agent principal (salon/bureau)  
cargo run --release -p symbion-agent-host

# Configuration interactive première fois
# 🏠 Domestic Agent Setup Wizard 🧙‍♂️
# 📡 MQTT connection test: ✅ Connected to domestic hub
# 🔍 Network discovery: Ethernet interface detected (stable)
# 🏷️  Agent ID: 7070fc0481d8 (MAC-based, persistent)
# 🏡 Location context: home_main_room
```

### 📱 **3. Interface Domestique (Tablette/Mobile)**
```bash
cd pwa-dashboard && npm run dev

# ✅ Dashboard domestique accessible :
# http://192.168.1.X:3001 - PWA adaptatif
# 🏠 domestic-control-widget : Contrôles maison
# 🧠 context-awareness-widget : Apprentissage habitudes  
# 📝 smart-notes-widget : Mémoire contextuelle
# 📊 home-metrics-widget : Télémétrie environnementale
```

---

## 🎯 Vision IoT/Domotique - (Roadmap)

### ✅ **Phase A - Infrastructure Intelligente (TERMINÉE)**
- ✅ Event Bus domotique MQTT pour communication inter-appareils
- ✅ Agents domestiques auto-découverte avec priorisation réseau stable  
- ✅ Plugin system pour modules de vie (cuisine, santé, finance, etc.)
- ✅ Interface adaptative selon contexte domestique (matin/soir/présence)

### 🚀 **Phase B - Automatisation Contextuelle (TERMINÉE)**
- ✅ Contrôle appareils domestiques (extinction/réveil machines)
- ✅ Context Engine avec détection automatique (nuit/week-end/semaine)
- ✅ Override manuel des modes avec expiration temporelle
- ✅ Notes intelligentes avec tags + icônes contextuels automatiques
- ✅ Interface mobile responsive avec tabs catégorisées
- ✅ Theming global dynamique selon mode contextuel

### 🏠 **Phase C - Maison Connectée (PROCHAINE)**
- ⏳ **Capteurs environnementaux** : Température, humidité, luminosité, présence
- ⏳ **Contrôle éclairage** : Philips Hue, variateurs, scénarios ambiance
- ⏳ **Thermostat intelligent** : Apprentissage préférences + optimisation énergie
- ⏳ **Sécurité domestique** : Caméras, détecteurs, notifications intrusion

### 🍳 **Phase D - Cuisine Intelligente**
- ⏳ **Frigo connecté** : Inventaire automatique + dates péremption + listes courses
- ⏳ **Suggestions menus** : IA selon restes + préférences + objectifs santé
- ⏳ **Électroménager smart** : Four, lave-vaisselle, machine à café programmables
- ⏳ **Assistant culinaire** : Recettes adaptatives + timer multiples + conseils

### 💰 **Phase E - Finance Personnelle Automatisée**
- ⏳ **Banque connectée** : Synchronisation comptes + catégorisation automatique
- ⏳ **Budget intelligent** : Alertes dépassement + optimisations dépenses
- ⏳ **Épargne automatique** : Virements selon revenus + objectifs personnalisés
- ⏳ **Investissements guidés** : Conseils IA + diversification + suivi performance

### 💪 **Phase F - Santé & Bien-être Intégré**
- ⏳ **Wearables connectés** : Fitness trackers + balance + tensiomètre
- ⏳ **Coaching adaptatif** : Exercices selon forme + météo + planning  
- ⏳ **Nutrition optimisée** : Menus selon objectifs + contraintes + goûts
- ⏳ **Sommeil intelligent** : Analyse cycles + optimisation environnement

### 🤝 **Phase G - Écosystème Familial**
- ⏳ **Multi-utilisateurs** : Profils personnalisés + préférences individuelles
- ⏳ **Coordination activités** : Planning partagé + répartition tâches + mood board
- ⏳ **Communication facilitée** : Messages contextuels + rappels + médiation

---

## 🔧 Technologies IoT Intégrées

### 📡 **Protocoles Domotiques**
- **MQTT** : Bus de communication principal entre appareils (déjà actif)
- **Zigbee/Z-Wave** : Capteurs et actionneurs bas niveau (phase C)
- **WiFi Smart** : Appareils connectés standards (Philips Hue, Sonos, etc.)
- **REST API** : Intégration services tiers (météo, calendrier, banques)

### 🧠 **Intelligence Contextuelle** 
- **Apprentissage automatique** : TensorFlow Lite pour patterns comportementaux
- **NLP basique** : Traitement langage naturel pour notes et commandes vocales  
- **Computer Vision** : Reconnaissance objets pour inventaire automatique
- **Géolocalisation** : Contexte lieu pour automatisations (maison/bureau/déplacements)

### 🔐 **Sécurité IoT**
- **Réseau isolé** : VLAN dédié appareils domestiques
- **Chiffrement bout-en-bout** : Communications MQTT + API sécurisées
- **Authentification forte** : Certificats appareils + rotation clés
- **Audit logging** : Traçabilité toutes actions automatiques

---