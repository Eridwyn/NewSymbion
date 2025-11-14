# Architecture Système Symbion

Vue d'ensemble de l'architecture IoT distribuée.

---

## 🏗️ Composants Principaux

### 🧬 symbion-kernel - Cerveau Central

**Rôle**: Centre nerveux de l'écosystème personnel

**Status**: ✅ Hub IoT Opérationnel + Monitoring Automatique
- Memory: 23.6MB
- MQTT: Connected
- Agents: 2+ actifs
- Surveillance: Cron 15 min + Alertes email Gmail

**Fonctionnalités**:
- ✅ **Event Bus MQTT** - Communication temps réel inter-appareils
- ✅ **Agent Registry** - Découverte et gestion appareils connectés
- ✅ **Plugin Orchestration** - Modules de vie (cuisine, santé, finance)
- ✅ **Context Engine** - Apprentissage habitudes et détection situations
- ✅ **API REST** - Interface pour contrôles et automatisations (90+ endpoints)
- ✅ **Health Monitoring** - Surveillance automatique + alertes proactives

### 🤖 symbion-agent-host - Assistants Domestiques

**Rôle**: Capteurs et actionneurs de l'environnement

**Status**: ✅ 2 Agents Actifs Multi-Environnement
- PC-Salon (Linux): Monitoring domestique + contrôle appareils
- PC-Bureau (Windows): Mode productivité + assistance professionnelle

**Fonctionnalités**:
- ✅ **Détection présence** - Activité système pour savoir si présent
- ✅ **Contrôle appareils** - Extinction/réveil machines selon contexte
- ✅ **Télémétrie environnementale** - Température, consommation, état matériel
- ✅ **Auto-découverte réseau** - Identification automatique appareils connectés
- ✅ **Adaptation contextuelle** - Comportement selon lieu (maison/bureau)

### 📱 pwa-dashboard - Interface Adaptative

**Rôle**: Miroir de l'écosystème qui s'adapte aux moments de la journée

**Status**: ✅ Interface Domestique Fonctionnelle
- URL: http://localhost:3000
- Widgets: Contrôles maison + Monitoring + Notes markdown + Santé système
- Mobile: Navigation fixe en bas + interface tactile optimisée

**Fonctionnalités**:
- ✅ **Widgets contextuels** - Interface change selon matin/soir/présence
- ✅ **Contrôles domestiques** - Gérer appareils connectés en un clic
- ✅ **Notes intelligentes** - Markdown rendering + expand/collapse + tags auto
- ✅ **PWA responsive** - Accessible tablette cuisine, smartphone, desktop
- ✅ **Navigation mobile fixe** - Tabs Contrôle/Système/Données ancrés en bas
- ✅ **Setup automatique certificat** - Téléchargement et vérification CA

### 📝 symbion-plugin-notes - Mémoire Externe

**Rôle**: Extension de la mémoire qui apprend les patterns

**Status**: ✅ Journal Contextuel Actif (2+ notes stockées)

**Fonctionnalités**:
- ✅ **Tags contextuels automatiques** - Selon SSID, heure, activité
- ✅ **Stockage distribué** - Notes accessibles sur tous appareils
- ✅ **Apprentissage habitudes** - Suggestions basées sur historique

---

## ⚡ Modes Contextuels Intelligents

### 👔 Symbion Cravate (Mode Professionnel)

**Détection**: SSID bureau + horaires 9h-18h + applications pro

**Fonctions IoT**:
- Focus mode avec notifications filtrées
- Préparation automatique notes clients/meetings
- Rappels pauses ergonomiques
- Optimisation éclairage/température bureau

### 🏡 Symbion Intime (Mode Domestique)

**Détection**: SSID domicile + soirée/weekend + apps loisir

**Fonctions IoT**:
- Suggestions repas selon frigo et restes
- Ambiance adaptive (éclairage selon humeur/météo)
- Contrôles entertainment et confort
- Coordination activités familiales

### 🌱 Symbion Neutre (Mode Surveillance)

**Toujours actif**: Maintenance et apprentissage continu

**Fonctions IoT**:
- Monitoring santé appareils domestiques
- Sauvegardes automatiques données personnelles
- Détection patterns comportementaux
- Optimisation énergétique silencieuse

---

## 🚀 Technologies IoT Intégrées

### 📡 Bus de Communication

- **MQTT**: Événements temps réel entre appareils (actif)
- **REST API**: Contrôles synchrones et intégrations externes (90+ endpoints)
- **WebSocket PWA**: Interface temps réel responsive
- **Contracts Registry**: Validation et versioning événements IoT

### 🧠 Intelligence Distribuée

- **Context Engine**: Détection SSID + horaires + patterns activité
- **Pattern Learning**: ML basique pour habitudes comportementales
- **Rule Engine**: Automatisations configurables (si-alors-action)
- **Semantic Tagging**: NLP basique pour catégorisation automatique

### 🔐 Sécurité Domestique

- **HTTPS/TLS Encryption**: Kernel HTTPS port 8443 (Let's Encrypt prod)
- **JWT Authentication**: Tokens JWT + bcrypt (cost factor 12)
- **Rate Limiting**: Protection API brute-force (5 req/sec par IP)
- **API Key Authentication**: Clé API secrète inter-services
- **Network Isolation**: Séparation appareils IoT du réseau principal
- **Device Authentication**: Certificats pour appareils de confiance
- **Command Validation**: Whitelist actions autorisées par contexte
- **Audit Trail**: Traçabilité complète automatisations domestiques

---

## 🎯 Expérience Utilisateur

### 🌅 Matin
1. Symbion détecte réveil via activité système/réseau
2. Prépare automatiquement agenda + météo sur dashboard
3. Suggère petit-déjeuner selon frigo + préférences apprises
4. Active mode productivité si jour de travail détecté

### 🏢 Bureau
1. Détection SSID professionnel → Mode Cravate activé
2. Notifications personnelles filtrées automatiquement
3. Préparation notes clients/réunions selon planning
4. Rappels pauses ergonomiques + optimisation environnement

### 🏠 Retour Maison
1. Géolocalisation/SSID → Mode Intime activé
2. Suggestions menu selon restes + envies + objectifs santé
3. Ambiance adaptive (éclairage/température) selon humeur/météo
4. Interface tablette cuisine avec contrôles domestiques

### 🌙 Nuit
1. Sauvegarde automatique journée (notes + finances + santé)
2. Préparation lendemain selon agenda + habitudes apprises
3. Optimisation environnement sommeil (température, silence)
4. Mode surveillance énergétique nocturne

---

## 📚 Modules de Vie (Roadmap)

### 🍳 Module Cuisine (Phase C)
- Frigo connecté + inventaire automatique
- Suggestions repas IA selon restes + santé + goûts
- Électroménager programmable
- Assistant culinaire avec recettes adaptatives

### 💰 Module Finance (Phase D)
- Synchronisation banques + catégorisation automatique
- Budget intelligent avec alertes + optimisations
- Épargne automatique selon revenus + objectifs
- Conseil investissements basé sur profil

### 💪 Module Santé (Phase E)
- Coaching adaptatif selon forme + météo + planning
- Nutrition optimisée selon objectifs + préférences
- Sommeil intelligent + optimisation environnement
- Métriques holistiques (activité/humeur/productivité)

### 🤝 Module Famille (Phase F)
- Multi-utilisateurs avec profils personnalisés
- Coordination activités + planning partagé
- Communication contextuelle + médiation automatique
- Listes collaboratives intelligentes

---

## 🛠️ Installation

### 1. Hub Central (Une machine fixe)
```bash
cd NewSymbion/symbion-kernel
SYMBION_API_KEY="your-key" cargo run
```

### 2. Agents Domestiques (Un par pièce/contexte)
```bash
cargo run --release -p symbion-agent-host
```

### 3. Interface (Tablette/Mobile)
```bash
cd pwa-dashboard && npm run dev
# Dashboard: http://localhost:3000
```

---

Voir aussi:
- [PHILOSOPHY.md](../PHILOSOPHY.md) - Principes architecturaux
- [QUICK_REFERENCE.md](../QUICK_REFERENCE.md) - API et commandes
- [CODE_STANDARDS.md](../CODE_STANDARDS.md) - Normes de développement
