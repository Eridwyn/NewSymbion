## 🌱 Phase A — Spine & DevKit

### 1. Kernel 0.1 🧬

Event Bus local (MQTT/IPC)

Contract Registry 📜

Data Ports v1 : memo, journal, hosts

Control API (/plugins, /contracts, /health)



### 2. Plugin Manager 🧩

Chargement/retrait à chaud 🔥

Sandbox + logs + healthcheck

Rollback & safe-mode 🛡️



### 3. DevKit 🛠️

Gabarits plugin Rust

Tests contractuels auto ✅

Stubs pour bus + ports



### 4. PWA seed 📱

Dashboard minimal

Widgets dynamiques (manifest-driven)





---

## 👕 Phase B — Noyau utile min (Hoodie)

### 5. Plugin Hosts 💻

Heartbeat @v2 (CPU, RAM, IP…)

Action Wake-on-LAN (WOL) ⚡

Widget état PC



### 6. Plugin Memo/Rappels 📝

memo.created@v1

Règles contextuelles simples (SSID, heure, cooldown) ⏰



### 7. Journal Auto 📖

journal.event@v1 unifié

Port port.journal.v1

Timeline front visuelle





---

## 🎨 Phase C — Palette & Routines

### 8. Palette universelle 🎛️

Commandes exposées via manifest

Ex: wake host, note "...", triage ip


### 9. Routines 🔄

DSL YAML steps (wake, wait_ping, http, open)

Plugins peuvent enregistrer des steps custom





---

## 💡 Phase D — Modules valeur quotidienne

### 10. Finance v1 💰

port.finance.v1, tx + budgets

Import CSV 📊

Widget résumé mois



### 11. Sélection Sport/Repas 🏋️🥗

Plugins indépendants (workout.selector, recipe.selector)

Produisent suggestion.prepared@v1



### 12. Cuisine-lite 🍳

port.pantry.v1 simple

Inventaire de base sans péremption

Liaison souple avec recettes





---

## 🛰️ Phase E — Contexte riche & Cravate

### 13. Context Engine v2 🧭

context.updated@v2 (place, horaire, réseau, busy mode)

Sources: SSID, LAN, horaires — GPS optionnel



### 14. Cravate v1 👔 (Pro)

Plugin calendar.bridge (ICS/Outlook import)

Plugin clients pour fiches entreprises

Publie meeting.context@v1

Rappels intelligents sur planning





---

## 👨‍👩‍👧‍👦 Phase F — Famille & Partage

### 15. Accounts & Spaces 🗝️

Multi-utilisateurs

Permissions par topic/port

Espaces: personal, shared.family



### 16. Famille v1 ❤️

Listes partagées 🛒

Mood board (vert/jaune/rouge) 🌡️

Widgets “collab” visibles dans l’espace partagé





---

## 🚀 Phase G — Finition durable

### 17. Tablette kiosque 📟

PWA offline-first

Widgets auto-layout



### 18. Observabilité 🔍

Traces événementielles

Relecture/replay des journées



### 19. IA locale optionnelle 🤖

Plugin advisor consommant events

Génère advice@v1

Débrayable : rien ne casse s’il est absent





---

👉 Chaque plugin = organe interchangeable.
👉 Le Kernel = colonne vertébrale nerveuse.
👉 Le front = miroir modulable.
