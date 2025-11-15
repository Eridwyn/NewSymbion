# 🔐 Security Documentation

**Last Updated**: 14 November 2025

Cette section regroupe toute la documentation sécurité de Symbion, organisée par type de contenu.

---

## 📂 Structure

```
docs/security/
├── procedures/         # Procédures opérationnelles (rotation secrets, incident response)
├── audits/            # Rapports d'audit et analyses de vulnérabilités
├── implementation/    # Guides d'implémentation (authentification, CSRF, rate limiting)
└── archive/           # Travaux complétés (phases de hardening terminées)
```

---

## 🚀 Quick Start

### Pour les Opérations

**Rotation des secrets** (tous les 90 jours):
→ `procedures/SECRETS_ROTATION_PROCEDURE.md`

**Réponse à incident sécurité**:
→ `procedures/INCIDENT_RESPONSE.md` *(coming soon)*

### Pour le Développement

**Implémenter l'authentification**:
→ `implementation/AUTHENTICATION.md`

**Ajouter protection CSRF**:
→ `implementation/CSRF_PROTECTION.md`

**Configurer rate limiting**:
→ `implementation/RATE_LIMITING.md`

### Pour les Audits

**Dernier audit complet**:
→ `audits/SECURITY_AUDIT_2025-11-12.md`

**Phase 2 Hardening (terminée)**:
→ `archive/SECURITY_HARDENING_PHASE2.md`

---

## 🛡️ Défense en Profondeur (6 Couches)

Symbion implémente une architecture de sécurité multi-niveaux:

1. **TLS Encryption** - HTTPS/TLS 1.3 avec certificats (port 8443)
2. **Authentication** - JWT tokens avec bcrypt (cost 12)
3. **Authorization** - Role-based access control (admin/user/guest)
4. **CSRF Protection** - Nonces single-use avec TTL 5 minutes
5. **Rate Limiting** - 5 tentatives login / 15 min par username
6. **Audit Trail** - Logs structurés avec rétention 1 an

Documentation détaillée: `implementation/DEFENSE_IN_DEPTH.md`

---

## 🔑 Secrets Management

### Secrets Actifs

| Secret | Location | Format | Rotation |
|--------|----------|--------|----------|
| `SYMBION_JWT_SECRET` | `.env` | 128 hex (64 bytes) | 90 jours |
| `SYMBION_API_KEY` | `.env` | 64 hex (32 bytes) | 90 jours |
| TLS Certificates | `/etc/mosquitto/certs/` | PEM | 365 jours |

**Dernière rotation**: 14 Novembre 2025
**Prochaine rotation**: 12 Février 2026

**Procédure**: `procedures/SECRETS_ROTATION_PROCEDURE.md`

---

## 📊 État de la Sécurité

### Derniers Audits

| Date | Type | Vulnérabilités | Status |
|------|------|----------------|--------|
| 2025-11-12 | Audit complet | 4 CRITICAL | ✅ **Toutes résolues** |
| 2025-11-14 | Phase 2 Hardening | 5 tâches | ✅ **100% terminé** |

**Score actuel**: 🟢 **10/10** (aucune vulnérabilité critique active)

### Prochaines Actions

- [ ] **Phase 3**: Implémentation HSTS + CSP headers (VULN-007, VULN-008)
- [ ] **Monitoring**: Alertes Prometheus pour détection anomalies
- [ ] **Backup codes MFA**: Validation côté login (actuellement génération seule)

---

## 📚 Index Complet

### Procédures (`procedures/`)

- `SECRETS_ROTATION_PROCEDURE.md` - Rotation secrets tous les 90 jours
- `INCIDENT_RESPONSE.md` *(planned)* - Réponse compromission secrets
- `TLS_CERTIFICATE_RENEWAL.md` *(planned)* - Renouvellement certificats

### Audits (`audits/`)

- `SECURITY_AUDIT_2025-11-12.md` - Audit complet 4 CRITICAL vulns
- `PENETRATION_TEST_2025-XX-XX.md` *(planned)* - Test d'intrusion externe

### Implémentation (`implementation/`)

- `AUTHENTICATION.md` - JWT + bcrypt + MFA/TOTP
- `CSRF_PROTECTION.md` - Nonces single-use TTL 5 min
- `RATE_LIMITING.md` - Protection brute-force
- `DEFENSE_IN_DEPTH.md` - Architecture 6 couches
- `TLS_CONFIGURATION.md` - HTTPS setup + certificats

### Archive (`archive/`)

- `SECURITY_HARDENING_PHASE2.md` - Phase 2 terminée (14 Nov 2025)
- `INCIDENT-2025-10-15-mqtt-not-connected.md` - Résolu

---

## 🔗 Références Externes

### Standards et RFCs

- **JWT**: [RFC 7519](https://tools.ietf.org/html/rfc7519)
- **TOTP**: [RFC 6238](https://tools.ietf.org/html/rfc6238)
- **bcrypt**: [OpenWall Spec](https://www.openwall.com/crypt/)

### Guides de Sécurité

- **OWASP Top 10**: [owasp.org/www-project-top-ten](https://owasp.org/www-project-top-ten/)
- **OWASP CSRF Prevention**: [cheatsheetseries.owasp.org](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- **NIST Password Guidelines**: [NIST SP 800-63B](https://pages.nist.gov/800-63-3/sp800-63b.html)

---

## 📞 Contact

**Security Contact**: markchavatte@gmail.com
**Responsible Disclosure**: Signaler les vulnérabilités via email chiffré (GPG key on request)

---

**Maintenu par**: Mark (avec assistance Claude Code)
