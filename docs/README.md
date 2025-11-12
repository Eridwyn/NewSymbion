# Documentation Symbion

> 📚 Documentation complète de l'architecture Symbion - Hub IoT domestique intelligent

## 🗂️ Structure de la Documentation

### 📡 [API HTTP](./api/README.md)
Documentation complète de l'API REST du Kernel Symbion
- **[Endpoints](./api/endpoints.md)** - Référence complète des 90+ endpoints
- **[Authentification](./api/authentication.md)** - JWT, MFA, WebAuthn, Device Trust
- **[WebAuthn/Passkeys](./api/webauthn.md)** - Guide complet authentification biométrique
- **[Sécurité](./api/security.md)** - CSRF, Rate Limiting, CORS

### 🔌 [MQTT](./mqtt/README.md)
Architecture de communication temps réel inter-composants
- **[Topics](./mqtt/topics.md)** - 13 topics avec structures de messages
- **[Contracts](./mqtt/contracts.md)** - Schémas JSON et validation
- **[Flows](./mqtt/flows.md)** - Patterns de communication

### 🛡️ Sécurité Globale

Symbion implémente 5 couches de sécurité :

1. **Authentification** - JWT (HS256, 24h) + API Keys fallback
2. **MFA** - TOTP (RFC 6238) pour opérations sensibles
3. **WebAuthn** - Passkeys biométriques (FIDO2)
4. **CSRF Protection** - Nonces one-time (5 min TTL)
5. **Rate Limiting** - Protection brute-force via `tower_governor`

### 🚀 Quick Start

**Cheat Sheet** : [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) - Référence rapide pour développeurs

```bash
# Consulter rapidement la documentation
./scripts/docs-lookup.sh              # Menu principal
./scripts/docs-lookup.sh endpoints    # Liste tous les endpoints HTTP
./scripts/docs-lookup.sh mqtt         # Liste tous les topics MQTT
./scripts/docs-lookup.sh security     # Résumé sécurité
./scripts/docs-lookup.sh auth         # Guide authentification
./scripts/docs-lookup.sh webauthn     # Passkeys biométriques
./scripts/docs-lookup.sh search "JWT" # Recherche dans la doc
```

## 🏗️ Architecture Globale

```
┌─────────────────────────────────────────┐
│  symbion-kernel (Cerveau Central)      │
│  - API REST (Axum)                      │
│  - MQTT Listener (Rumqttc)             │
│  - Context Engine                       │
│  - Agent Registry                       │
│  - Decision Engine (PR3)                │
└─────────────────────────────────────────┘
              ↓ MQTT
    ┌─────────┴─────────┐
    ↓                   ↓
┌─────────┐       ┌─────────────┐
│ Agents  │       │  Plugins    │
│ Hosts   │       │  (Notes)    │
└─────────┘       └─────────────┘
              ↓ HTTP/WS
    ┌─────────────────────┐
    │  PWA Dashboard      │
    └─────────────────────┘
```

## 📝 Maintenance Documentation

**Règle obligatoire** : À chaque session de développement :

1. ✅ Mettre à jour `docs/api/endpoints.md` si nouveaux endpoints
2. ✅ Mettre à jour `docs/mqtt/topics.md` si nouveaux topics
3. ✅ Mettre à jour `docs/api/security.md` si nouvelles mesures sécurité
4. ✅ Mettre à jour `CLAUDE.md` avec références vers documentation modifiée

**Checklist avant commit** :
- [ ] Documentation API à jour
- [ ] Documentation MQTT à jour
- [ ] Documentation sécurité à jour
- [ ] CLAUDE.md référence correctement la doc

## 🔍 Recherche Rapide

**Par catégorie** :
- Authentification → `docs/api/authentication.md`
- Gestion agents → `docs/api/endpoints.md#agents`
- Communication temps réel → `docs/mqtt/topics.md`
- Sécurité → `docs/api/security.md`

**Par cas d'usage** :
- "Comment créer un nouvel endpoint ?" → `docs/api/endpoints.md` (voir pattern existant)
- "Comment publier un événement MQTT ?" → `docs/mqtt/topics.md` (voir exemples)
- "Comment implémenter MFA ?" → `docs/api/authentication.md#mfa`
- "Quel topic pour envoyer une commande agent ?" → `docs/mqtt/topics.md#kernel-to-agents`

## 📊 Statistiques

- **Endpoints HTTP** : 90+
- **Topics MQTT** : 13
- **Mécanismes sécurité** : 5
- **Plugins actifs** : 1 (Notes/Memo)
- **Agents connectés** : 2 (Salon Linux, Bureau Windows)

---

**Dernière mise à jour** : 2025-11-12
**Version Kernel** : 0.1.0
**Maintenu par** : Claude Code
