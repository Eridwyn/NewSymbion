# Architecture Symbion - Documentation

## 📋 Référence Priorité Actuelle

**Document de référence principal** : [P0-ROADMAP-FINAL.md](./P0-ROADMAP-FINAL.md)

Ce document contient la roadmap P0 complète et verrouillée pour l'implémentation du Decision Engine et Context Engine de Symbion.

## 🎯 Priorité P0 - 6 PRs Séquentielles

1. **PR1** : context/timezone+hysteresis → v0.2.0-alpha.1
2. **PR2** : api/v1+auth+MFA+nonce → v0.2.0-alpha.2
3. **PR3** : decision/guards-first+weights → v0.2.0-beta.1
4. **PR4** : observability-min → v0.2.1
5. **PR5** : fail-safe → v0.2.2
6. **PR6** : intentions/lifecycle → v0.2.3 ✅ PRODUCTION-READY

## 📅 Timeline

- **Semaine 1** : PR1 + PR2
- **Semaine 2** : PR3 + PR4
- **Semaine 3** : PR5 + PR6
- **Semaine 4** : Tests intégration + polish

## 🔒 Principes Verrouillés

- Context Engine = Source de vérité unique pour le mode
- Decision Engine = Lecteur read-only + validateur
- Guards évalués AVANT trust_score (zéro bypass)
- Intentions lifecycle complet (offline check, timeouts, conflicts, notifications, persistence)

## 📖 Consulter la Référence

Voir [P0-ROADMAP-FINAL.md](./P0-ROADMAP-FINAL.md) pour :
- Exigences techniques détaillées
- Code samples complets
- Tests à livrer
- Configuration requise
- Points de rigueur (horloges monotones, verrouillages, backoff MQTT)

---

*Dernière mise à jour : 27 Octobre 2025*
