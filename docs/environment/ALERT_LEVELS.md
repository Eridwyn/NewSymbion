# Environment Alert Levels (F1)

## Vue d'ensemble

Le système d'alerte environnemental de Symbion utilise un modèle **basé sur la physique du point de rosée** (Magnus formula) pour détecter les risques de condensation et de moisissure.

**Fichier source**: `symbion-kernel/src/dew_point_alerts.rs`
**Feature**: F1 - Environment Monitoring
**Dernière mise à jour**: 19 Novembre 2025

---

## 🌡️ Les 6 Niveaux d'Alerte

### 1. Safe ✅ - Conditions normales

**Conditions de déclenchement**:
- RH ≤ 55% **OU**
- Temps insuffisant pour valider les autres seuils (< 90% coverage temporelle)

**Message**: "Conditions normales"
**Action suggérée**: Aucune action requise
**Impact**: Aucun
**Emoji PWA**: ✅

---

### 2. Weak 💧 - Humidité en tendance haute

**Conditions de déclenchement**:
- **RH > 55%** pendant **6 heures consécutives**
- Validation temporelle: ≥90% de couverture sur les 6 heures

**Message**: "Humidité en tendance haute"
**Action suggérée**: Surveiller, aérer préventivement
**Impact**: Low
**Emoji PWA**: 💧

**Cas d'usage**: Détecte les tendances haute humidité avant qu'elles deviennent problématiques (ex: plusieurs personnes dans une pièce fermée pendant longtemps).

---

### 3. Moderate 💦 - Humidité excessive prolongée

**Conditions de déclenchement**:
- **RH > 60%** pendant **3 heures consécutives**
- Validation temporelle: ≥90% de couverture sur les 3 heures

**Message**: "Humidité excessive prolongée"
**Action suggérée**: Ventilation recommandée
**Impact**: Medium
**Emoji PWA**: 💦

**Cas d'usage**: Humidité trop haute pendant trop longtemps, risque de confort dégradé et début de conditions favorables à la moisissure.

---

### 4. Strong 🌊 - Risque de condensation

**Conditions de déclenchement** (l'une des deux):
- **RH > 65%** pendant **1 heure** **OU**
- **ΔT < 3°C** pendant **1 heure**
  - ΔT = T_surface - T_dew (écart entre température surface et point de rosée)
  - T_surface estimée = T_air - 3°C (configurable)

**Message**: "Risque de condensation"
**Action suggérée**: Ventilation urgente, surveillance surfaces froides
**Impact**: Medium
**Emoji PWA**: 🌊

**Cas d'usage**: Détecte les conditions où la condensation devient physiquement probable (surfaces froides proches du point de rosée).

---

### 5. Critical ⚠️ - Condensation très probable

**Conditions de déclenchement** (l'une des deux):
- **RH > 70%** pendant **20 minutes** **OU**
- **ΔT < 2°C** pendant **20 minutes**

**Message**: "Condensation très probable"
**Action suggérée**: Ventilation immédiate, déshumidificateur
**Impact**: High
**Emoji PWA**: ⚠️

**Cas d'usage**: Conditions critiques où la condensation va probablement se former sous peu (ex: cuisine avec vapeur d'eau, salle de bain après douche).

---

### 6. Danger 🚨 - Condensation certaine

**Conditions de déclenchement** (l'une des deux):
- **RH > 75%** pendant **5 minutes** **OU**
- **ΔT ≤ 0°C** pendant **5 minutes** (température surface = température point de rosée)

**Message**: "Condensation certaine / surfaces humides"
**Action suggérée**: Intervention urgente, risque moisissure immédiat
**Impact**: High
**Emoji PWA**: 🚨

**Cas d'usage**: Condensation physiquement certaine, surfaces probablement déjà mouillées. Risque immédiat de développement de moisissures sous 24-48h.

---

## 🔬 Base Physique - Point de Rosée

### Formule de Magnus

Le point de rosée (T_dew) est calculé via la formule de Magnus (précision ±0.4°C):

```
γ(T,RH) = ln(RH/100) + (a × T) / (b + T)
T_dew = (b × γ) / (a - γ)
```

**Constantes par défaut**:
- `a = 17.62`
- `b = 243.12°C`

### Delta T (ΔT)

L'écart entre la température de surface et le point de rosée indique la proximité de la condensation:

```
ΔT = T_surface - T_dew
```

**Interprétation**:
- ΔT > 3°C: Sûr, condensation impossible
- ΔT < 3°C: Pré-condensation, surveiller
- ΔT < 2°C: Condensation très probable
- ΔT ≤ 0°C: Condensation certaine (température surface atteinte le point de rosée)

**Estimation T_surface**:
```
T_surface ≈ T_air - offset
```
- Offset par défaut: **3.0°C** (murs extérieurs non isolés)
- Configurable selon le type de surface (fenêtres, murs, etc.)

---

## ⚙️ Configuration

### DewPointAlertConfig

Tous les seuils sont configurables via `DewPointAlertConfig` (`symbion-kernel/src/dew_point_alerts.rs:32-63`):

```rust
pub struct DewPointAlertConfig {
    // Constantes Magnus
    pub magnus_a: f32,                    // Default: 17.62
    pub magnus_b: f32,                    // Default: 243.12

    // Estimation surface
    pub surface_temp_offset: f32,         // Default: 3.0°C

    // Seuil Weak
    pub weak_rh_threshold: f32,           // Default: 55.0%
    pub weak_duration_hours: f32,         // Default: 6.0h

    // Seuil Moderate
    pub moderate_rh_threshold: f32,       // Default: 60.0%
    pub moderate_duration_hours: f32,     // Default: 3.0h

    // Seuil Strong
    pub strong_rh_threshold: f32,         // Default: 65.0%
    pub strong_delta_t: f32,              // Default: 3.0°C
    pub strong_duration_hours: f32,       // Default: 1.0h

    // Seuil Critical
    pub critical_rh_threshold: f32,       // Default: 70.0%
    pub critical_delta_t: f32,            // Default: 2.0°C
    pub critical_duration_minutes: u32,   // Default: 20 min

    // Seuil Danger
    pub danger_rh_threshold: f32,         // Default: 75.0%
    pub danger_delta_t: f32,              // Default: 0.0°C
    pub danger_duration_minutes: u32,     // Default: 5 min
}
```

---

## 🕐 Validation Temporelle et Hysteresis (90% Double Threshold)

**Problème résolu**: Faux positifs et clignotement d'alertes

**Solution** (commit `0c365ac` + `a45e378` + `[nouveau]`):

### 1. Validation Coverage Temporelle (90%)
- Chaque seuil temporel requiert **≥90% de couverture** de la durée
- Exemple: Pour "weak" (6h), il faut au minimum **5h24 de données réelles** (90% de 6h)
- Empêche les alertes "weak" avec seulement 35 min de données

### 2. Hysteresis des Lectures (90%)
- **≥90% des lectures** dans la fenêtre doivent être > seuil pour déclencher l'alerte
- **<90% des lectures** dans la fenêtre pour que l'alerte disparaisse
- Tolérance aux variations temporaires (10% de marge)

**Implémentation**:
```rust
// Validation coverage temporelle
let actual_duration = newest_timestamp - oldest_timestamp;
let required_duration = duration * 90%; // 90% coverage minimum

if actual_duration < required_duration {
    return false; // Pas assez de données - pas d'alerte
}

// Validation hysteresis des lectures
let percentage_above = readings_above_threshold / total_readings;
percentage_above >= 0.90  // 90% des lectures doivent être > seuil
```

**Avantages**:
- ✅ Pas de clignotement d'alerte avec variations temporaires
- ✅ Disparition progressive quand amélioration durable
- ✅ Alerte persiste malgré quelques lectures sous seuil
- ✅ Cohérent avec validation temporelle (même seuil 90%)

**Fichiers**:
- `symbion-kernel/src/environment.rs:148-187` (`is_humidity_sustained()`)
- `symbion-kernel/src/dew_point_alerts.rs:387-440` (`is_delta_t_sustained_below()`)

---

## 📊 Ordre de Priorité

Les niveaux sont évalués du **plus critique au moins critique**:

```
Danger > Critical > Strong > Moderate > Weak > Safe
```

**Logique**: Si plusieurs conditions sont remplies simultanément (ex: RH=76% remplit Danger, Critical, Strong, Moderate, Weak), seul le **niveau le plus élevé** est retourné (Danger).

**Implémentation**: `symbion-kernel/src/dew_point_alerts.rs:251-301` (`DewPointCalculator::evaluate()`)

---

## 🔗 API & Endpoints

### GET `/v1/environment/{room_id}`

Retourne le status d'alerte actuel:

```json
{
  "room_id": "chambre",
  "current": {
    "temperature_c": 21.8,
    "humidity_pct": 57.0,
    "timestamp": "2025-11-19T21:39:54Z"
  },
  "status": "safe",
  "history": [ /* 137 readings */ ]
}
```

**Valeurs possibles de `status`**:
- `"safe"`
- `"weak"`
- `"moderate"`
- `"strong"`
- `"critical"`
- `"danger"`

**Référence complète**: [docs/api/endpoints.md](../api/endpoints.md)

---

## 🧪 Tests

**Fichiers de tests**:
- `symbion-kernel/src/environment.rs:168-431` (24 tests)
- `symbion-kernel/src/decision/environment.rs:168-431` (11 tests)
- `symbion-kernel/src/dew_point_alerts.rs:442-690` (13 tests)

**Total**: **48 tests** couvrant tous les niveaux d'alerte

**Exemples de tests**:
- `test_evaluate_dew_point_alert_weak`: RH 58% pendant 7h → "weak"
- `test_evaluate_dew_point_alert_critical`: RH 72% pendant 20min → "critical"
- `test_is_humidity_sustained_false_insufficient_data`: 35min de données pour seuil 6h → false

---

## 📈 Historique & Migration

### Ancien système (avant F1 refactor)

**Problèmes**:
- Seuils arbitraires non basés sur physique
- Pas de validation temporelle → faux positifs fréquents
- Statut "humid" / "risk_mold" peu informatif

### Nouveau système (F1 Enhanced - Nov 2025)

**Améliorations**:
- 🔬 Basé sur formule Magnus (point de rosée)
- ⏱️ Validation temporelle 90% coverage
- 📊 6 niveaux progressifs avec actions claires
- ✅ 48 tests unitaires
- 🔄 Recalcul automatique au chargement (persistence fix)

**Commits clés**:
- `9d118f8`: Refactor vers modèle dew point
- `0c365ac`: Fix temporal validation (90% coverage)
- `a45e378`: Fix persistence status recalculation

---

## 🎯 Cas d'Usage Réels

### Scénario 1: Chambre occupée la nuit
- **20:00**: 2 personnes entrent, RH commence à monter (48% → 52%)
- **02:00**: RH atteint 57% depuis 6h → **Alerte "weak"** 💧
- **Action**: Ouvrir fenêtre 5 min le matin

### Scénario 2: Salle de bain après douche
- **07:00**: Douche chaude, RH monte rapidement (45% → 78%)
- **07:05**: RH > 75% depuis 5 min → **Alerte "danger"** 🚨
- **Action**: VMC forcée + fenêtre ouverte immédiatement

### Scénario 3: Cuisine en hiver (surfaces froides)
- **12:00**: Cuisson vapeur, T=19°C, RH=68%
- **Calcul**: T_dew=13°C, T_surface=16°C → ΔT=3°C
- **12:30**: ΔT < 3°C pendant 30min → **Alerte "strong"** 🌊
- **Action**: Hotte aspirante + surveillance fenêtres froides

---

## 📚 Voir Aussi

- [Architecture F1 Environment](../architecture/SYSTEM_OVERVIEW.md#f1-environment)
- [API Endpoints](../api/endpoints.md)
- [MQTT Topics Environment](../mqtt/topics.md#environment-topics)
- [Roadmap F1](../roadmap_2025-11.md#f1---environment)

---

**Dernière mise à jour**: 19 Novembre 2025
**Version**: 1.1.7
**Auteur**: Claude Code + eridwyn
