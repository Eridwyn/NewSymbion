# 🗺️ NewSymbion Roadmap

**Système modulaire d'automatisation personnelle avec agents distribués**

---

## ✅ Phase A — Spine & DevKit (TERMINÉE)

### ✅ 1. Kernel 0.1 🧬

✅ Event Bus MQTT avec rumqttc 0.24.0  
✅ Contract Registry avec 7 contrats MQTT + 5 HTTP  
✅ Data Ports architecture (migration vers plugins)  
✅ Control API REST sécurisée avec API key obligatoire  
✅ 20+ endpoints : /health, /system/health, /plugins, /contracts, /agents, /ports/memo  

### ✅ 2. Plugin Manager 🧩

✅ Chargement/arrêt à chaud avec circuit breaker  
✅ Health monitoring continu des plugins  
✅ Rollback automatique en cas d'échec  
✅ API REST : /plugins/{name}/start|stop|restart  

### ✅ 3. DevKit 🛠️

✅ Scaffold automatique : `devkit/scaffold-plugin.py`  
✅ Tests contractuels : `devkit/contract-tester.py`  
✅ Bibliothèque `symbion-devkit` avec MockMqttClient  
✅ Templates Rust complets pour nouveaux plugins  

### ✅ 4. PWA Dashboard 📱

✅ Lit + Vite + PWA avec service workers  
✅ Dashboard temps réel avec widgets dynamiques  
✅ Integration API REST + MQTT WebSocket  
✅ Responsive design mobile-first  
✅ Widgets : system-health, plugins, notes, agents-network  

---

## 🚀 Phase B — Noyau utile (EN COURS)

### ✅ 5. Agents System 🤖

✅ symbion-agent-host multi-OS (Linux/Windows/Android)  
✅ Auto-découverte MAC/IP avec priorité Ethernet  
✅ Contracts MQTT : agents.registration@v1, agents.command@v1, agents.heartbeat@v1  
✅ Capacités système : shutdown, reboot, processus, métriques  
✅ Service systemd pour auto-start  
✅ Persistance centralisée dans data/agents.json  
⏳ API REST /agents avec contrôle système à distance  
⏳ PWA widgets : agent-control-widget modal détaillé  

### ✅ 6. Plugin Notes 📝

✅ Système distribué via MQTT (notes.command@v1, notes.response@v1)  
✅ API bridge /ports/memo 100% compatible  
✅ CRUD complet avec métadonnées (urgent, context, tags)  
✅ PWA widget notes intégré  
⏳ Règles contextuelles (SSID, heure, cooldown)  

### ⏳ 7. Journal Auto 📖

⏳ journal.event@v1 unifié  
⏳ Timeline front visuelle avec filtres  
⏳ Auto-capture événements système  

---

## 🎨 Phase C — Palette & Routines (PLANIFIÉ)

### 8. Palette universelle 🎛️

⏳ Commandes exposées via manifest  
⏳ Ex: wake host, note "...", triage ip  
⏳ Interface command palette PWA  

### 9. Routines 🔄

⏳ DSL YAML steps (wake, wait_ping, http, open)  
⏳ Plugins peuvent enregistrer des steps custom  
⏳ Scheduling et triggers contextuels  

---

## 💡 Phase D — Modules valeur quotidienne (FUTUR)

### 10. Finance v1 💰

⏳ port.finance.v1, tx + budgets  
⏳ Import CSV 📊  
⏳ Widget résumé mois  

### 11. Sélection Sport/Repas 🏋️🥗

⏳ Plugins indépendants (workout.selector, recipe.selector)  
⏳ Produisent suggestion.prepared@v1  

### 12. Cuisine-lite 🍳

⏳ port.pantry.v1 simple  
⏳ Inventaire de base sans péremption  
⏳ Liaison souple avec recettes  

---

## 🛰️ Phase E — Contexte riche & Pro (FUTUR)

### 13. Context Engine v2 🧭

⏳ context.updated@v2 (place, horaire, réseau, busy mode)  
⏳ Sources: SSID, LAN, horaires — GPS optionnel  

### 14. Module Pro 👔

⏳ Plugin calendar.bridge (ICS/Outlook import)  
⏳ Plugin clients pour fiches entreprises  
⏳ Publie meeting.context@v1  
⏳ Rappels intelligents sur planning  

---

## 👨‍👩‍👧‍👦 Phase F — Famille & Partage (FUTUR)

### 15. Accounts & Spaces 🗝️

⏳ Multi-utilisateurs  
⏳ Permissions par topic/port  
⏳ Espaces: personal, shared.family  

### 16. Famille v1 ❤️

⏳ Listes partagées 🛒  
⏳ Mood board (vert/jaune/rouge) 🌡️  
⏳ Widgets "collab" visibles dans l'espace partagé  

---

## 🚀 Phase G — Finition durable (FUTUR)

### 17. Tablette kiosque 📟

⏳ PWA offline-first  
⏳ Widgets auto-layout  
⏳ Interface tactile optimisée  

### 18. Observabilité 🔍

⏳ Traces événementielles  
⏳ Relecture/replay des journées  
⏳ Métriques et analytics  

### 19. IA locale optionnelle 🤖

⏳ Plugin advisor consommant events  
⏳ Génère advice@v1  
⏳ Débrayable : rien ne casse s'il est absent  

---

## 📊 État Actuel

**Architecture Production-Ready ✅**
- Workspace Rust avec 5 composants
- 25+ endpoints API REST sécurisés
- 7 contracts MQTT + 5 contracts HTTP
- PWA Dashboard temps réel responsive
- Agent multi-OS avec service systemd
- Plugin system avec hot loading
- DevKit complet pour développement

**Prochaines Étapes (Phase B completion)**
1. Finaliser API REST /agents avec contrôle système
2. Compléter widgets PWA agent-control-widget  
3. Implémenter règles contextuelles pour notes
4. Développer journal auto unifié
5. Tests cross-platform agents Windows/Android

---

*👉 Chaque plugin = organe interchangeable*  
*👉 Le Kernel = colonne vertébrale nerveuse*  
*👉 Le front = miroir modulable*