# Stratégie d'Authentification PWA - Device Pairing

## Contexte

La PWA Symbion nécessite une authentification pour accéder aux endpoints protégés du kernel.
Un fallback API key hardcodé n'est **pas une mesure de sécurité** - c'est une information publique.

## Solution Proposée: Device Pairing + Token Révocable

### Principe

1. **First Login**: L'utilisateur se connecte via passkey (WebAuthn) ou login/password
2. **Device Token**: Le serveur génère un token longue durée lié à cet appareil
3. **Storage**: Token stocké en localStorage (compromis UX accepté pour usage personnel)
4. **Révocation**: L'utilisateur peut révoquer n'importe quel device depuis les paramètres

### Flow d'Authentification

```
┌─────────────────────────────────────────────────────────────────┐
│                      PREMIER ACCÈS                               │
├─────────────────────────────────────────────────────────────────┤
│  1. PWA détecte: pas de token localStorage                      │
│  2. Affiche écran "Associer cet appareil"                       │
│  3. Options: Passkey (recommandé) / Login+Password              │
│  4. Auth réussie → Serveur génère device_token (JWT 1 an)       │
│  5. Stockage: localStorage.setItem('symbion_device_token', jwt) │
│  6. Device enregistré: nom auto (User-Agent), date, IP          │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      ACCÈS SUIVANTS                              │
├─────────────────────────────────────────────────────────────────┤
│  1. PWA lit token localStorage                                  │
│  2. Inclut dans header: Authorization: Bearer <token>           │
│  3. Kernel valide: signature + non-révoqué + non-expiré         │
│  4. Si invalide → retour écran pairing                          │
└─────────────────────────────────────────────────────────────────┘
```

### Endpoints Requis (Kernel)

```
POST /v1/auth/pair-device
  Body: { method: "passkey" | "password", credential: ... }
  Response: { device_token: "jwt...", device_id: "uuid", expires_at: timestamp }

GET /v1/auth/devices
  Response: [{ id, name, paired_at, last_used, current: bool }]

DELETE /v1/auth/devices/:id
  → Révoque le device (token invalide immédiatement)

POST /v1/auth/logout
  → Révoque le device courant
```

### Structure JWT Device Token

```json
{
  "sub": "user_id",
  "device_id": "uuid-device",
  "device_name": "Chrome/Linux",
  "iat": 1706000000,
  "exp": 1737536000,
  "scope": ["read", "write", "admin"]
}
```

### Révocation

- Table `revoked_devices` en base avec `device_id` + `revoked_at`
- Check sur chaque requête: `device_id NOT IN revoked_devices`
- Révocation instantanée (pas besoin d'attendre expiration JWT)

### Sécurité

| Aspect | Mesure |
|--------|--------|
| Token theft | Révocation possible + expiration 1 an |
| XSS | CSP strict + sanitization |
| CSRF | Token CSRF séparé pour mutations |
| Brute force | Rate limiting sur /auth/pair-device |
| Replay | Nonce dans passkey challenge |

### UI Mode Dégradé

Si pas de token valide, la PWA affiche:
- Dashboard en lecture seule (health status public)
- Message clair: "Appareil non associé"
- Bouton: "Associer cet appareil"

### Migration

1. Supprimer tous les fallbacks API key hardcodés ✅
2. Implémenter endpoints kernel `/v1/auth/pair-device` et `/v1/auth/devices`
3. Ajouter écran pairing dans PWA
4. Ajouter page "Mes appareils" dans paramètres

### Compromis Acceptés

- **localStorage**: Vulnérable à XSS mais acceptable pour usage domestique personnel
- **Token longue durée**: 1 an pour éviter re-auth fréquent, compensé par révocation
- **Pas de refresh token**: Simplicité > sécurité maximale pour ce use-case

---

**Status**: Proposition - En attente de validation utilisateur
**Date**: Février 2026
