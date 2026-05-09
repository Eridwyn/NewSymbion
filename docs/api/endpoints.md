# Référence Complète des Endpoints HTTP

> Généré automatiquement depuis l'OpenAPI spec du kernel (Symbion Kernel API v1.0.0).
> Source : `https://localhost:8443/api-docs/openapi.json` (Swagger UI : `/swagger-ui`)
> Dernière régénération : 2026-05-09

**115 paths uniques · 134 opérations HTTP · 11 tags**

---

## Sommaire par tag

- [Agents](#agents) — 28 endpoints
- [Authentication](#authentication) — 22 endpoints
- [Automations](#automations) — 10 endpoints
- [Context](#context) — 6 endpoints
- [Decision](#decision) — 14 endpoints
- [Environment](#environment) — 5 endpoints
- [Intelligence](#intelligence) — 15 endpoints
- [Modes](#modes) — 5 endpoints
- [Notifications](#notifications) — 11 endpoints
- [Schedule](#schedule) — 7 endpoints
- [System](#system) — 11 endpoints

---

## Agents

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/agents` | List all registered agents with their current status. | 🔐 bearer |
| `GET` | `/agents/{id}` | Return full details of a single agent by ID. | 🔐 bearer |
| `POST` | `/agents/{id}/command` | Execute a shell command on the specified agent. | 🔐 bearer |
| `GET` | `/agents/{id}/commands` | List pending commands for the specified agent. | 🔐 bearer |
| `POST` | `/agents/{id}/commands` | Submit a tracked command for execution on the agent. | 🔐 bearer |
| `POST` | `/agents/{id}/hibernate` | Send a hibernate (sleep) command to the specified agent. | 🔐 bearer |
| `GET` | `/agents/{id}/metrics` | Return real-time system metrics for an agent, or request them via MQTT. | 🔐 bearer |
| `POST` | `/agents/{id}/notify` | Send a desktop notification to the agent. | 🔐 bearer |
| `GET` | `/agents/{id}/plugins` | Return plugin data from agent heartbeat. | 🔐 bearer |
| `POST` | `/agents/{id}/plugins/{plugin_id}/command` | Send a command to an agent plugin. | 🔐 bearer |
| `GET` | `/agents/{id}/processes` | Return the process list for an agent, or request it via MQTT. | 🔐 bearer |
| `POST` | `/agents/{id}/processes/{pid}/kill` | Kill a specific process on the agent by PID. | 🔐 bearer |
| `POST` | `/agents/{id}/reboot` | Send a reboot command to the specified agent. | 🔐 bearer |
| `GET` | `/agents/{id}/scheduled-tasks` | List scheduled tasks on the agent. | 🔐 bearer |
| `POST` | `/agents/{id}/scheduled-tasks` | Create a scheduled task on the agent. | 🔐 bearer |
| `DELETE` | `/agents/{id}/scheduled-tasks/{name}` | Delete a scheduled task on the agent. | 🔐 bearer |
| `POST` | `/agents/{id}/screenshot` | Request a screenshot from the agent. | 🔐 bearer |
| `POST` | `/agents/{id}/shutdown` | Send a shutdown command to the specified agent. | 🔐 bearer |
| `GET` | `/agents/{id}/watchdog` | Return watchdog health report for an agent. | 🔐 bearer |
| `POST` | `/commands/{command_id}/cancel` | Cancel a pending or in-progress command. | 🔐 bearer |
| `GET` | `/commands/{command_id}/status` | Return the current status and output of a command. | 🔐 bearer |
| `DELETE` | `/environment/sensors/{sensor_id}` | Soft-delete a sensor (purged after 7 days). | 🔐 bearer |
| `DELETE` | `/v1/agents/{id}` | Soft-delete an agent (purged after 7 days). | 🔐 bearer |
| `POST` | `/v1/agents/{id}/reconnect` | Send a reconnect command to the specified agent via MQTT. | 🔐 bearer |
| `POST` | `/v1/plugins/{name}/restart` | POST /v1/plugins/:name/restart - Restart plugin via sudo systemctl restart (async, returns immediately) | 🔐 bearer |
| `POST` | `/v1/plugins/{name}/start` | POST /v1/plugins/:name/start - Start plugin via sudo systemctl start (async, returns immediately) | 🔐 bearer |
| `GET` | `/v1/plugins/{name}/status` | GET /v1/plugins/:name/status - Get plugin status via systemctl --user is-active | 🔐 bearer |
| `POST` | `/v1/plugins/{name}/stop` | POST /v1/plugins/:name/stop - Stop plugin via sudo systemctl stop (async, returns immediately) | 🔐 bearer |

## Authentication

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/auth/csrf/nonce` | GET /v1/auth/csrf/nonce - Générer un nonce CSRF pour l'utilisateur courant | 🔐 bearer |
| `POST` | `/auth/login` | POST /auth/login — Authenticate user with username/password, return JWT token with optional MFA and device trust. | 🔓 public |
| `POST` | `/auth/logout` | POST /auth/logout — Log out the current user (client-side token removal). | 🔐 bearer |
| `POST` | `/auth/mfa/disable` | POST /v1/auth/mfa/disable - Désactiver MFA pour l'utilisateur | 🔐 bearer |
| `POST` | `/auth/mfa/setup` | POST /v1/auth/mfa/setup - Initialiser la configuration MFA (génère secret + QR code) | 🔐 bearer |
| `GET` | `/auth/mfa/status` | GET /v1/auth/mfa/status - Vérifier si MFA est activé pour l'utilisateur courant | 🔐 bearer |
| `POST` | `/auth/mfa/verify` | POST /v1/auth/mfa/verify - Vérifier un code TOTP et activer MFA | 🔐 bearer |
| `POST` | `/auth/reload` | POST /auth/reload - Recharger les utilisateurs depuis users.json sans redémarrer le kernel | 🔐 bearer |
| `GET` | `/auth/session` | GET /auth/session — Retrieve current session information for the authenticated user. | 🔐 bearer |
| `GET` | `/auth/verify` | GET /auth/verify — Verify JWT token validity and return decoded claims. | 🔐 bearer |
| `POST` | `/auth/webauthn/authenticate-discoverable-start` | POST /auth/webauthn/authenticate-discoverable-start — Start passwordless authentication using discoverable credentials. | 🔓 public |
| `POST` | `/auth/webauthn/authenticate-finish` | POST /auth/webauthn/authenticate-finish — Complete passkey authentication and return a JWT token. | 🔓 public |
| `POST` | `/auth/webauthn/authenticate-start` | POST /auth/webauthn/authenticate-start — Start passkey authentication for a given username. | 🔓 public |
| `GET` | `/auth/webauthn/passkeys` | GET /auth/webauthn/passkeys - Lister les passkeys de l'utilisateur connecté | 🔐 bearer |
| `DELETE` | `/auth/webauthn/passkeys/{credential_id}` | DELETE /auth/webauthn/passkeys/:credential_id - Supprimer une passkey | 🔐 bearer |
| `POST` | `/auth/webauthn/register-finish` | POST /auth/webauthn/register-finish — Complete passkey registration and persist the credential. | 🔐 bearer |
| `POST` | `/auth/webauthn/register-start` | POST /auth/webauthn/register-start — Start passkey registration for the authenticated user. | 🔐 bearer |
| `GET` | `/ca-certificate` | GET /ca-certificate — Download the CA certificate as a PEM file. | 🔓 public |
| `GET` | `/v1/users` | GET /v1/users - Lister tous les utilisateurs (admin seulement, sans mots de passe) | 🔐 bearer |
| `POST` | `/v1/users` | POST /v1/users - Créer un nouvel utilisateur (admin seulement) | 🔐 bearer |
| `DELETE` | `/v1/users/{username}` | DELETE /v1/users/{username} - Supprimer un utilisateur (admin seulement) | 🔐 bearer |
| `PUT` | `/v1/users/{username}/password` | PUT /v1/users/{username}/password - Changer le mot de passe d'un utilisateur | 🔐 bearer |

## Automations

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/automations` | GET /v1/automations - List all automations | 🔐 bearer |
| `POST` | `/automations` | POST /v1/automations - Create automation | 🔐 bearer |
| `GET` | `/automations/history` | GET /v1/automations/history - Get execution history | 🔐 bearer |
| `GET` | `/automations/schema` | GET /v1/automations/schema - Get schema for rule builder | 🔐 bearer |
| `DELETE` | `/automations/{automation_id}` | DELETE /v1/automations/{id} - Soft-delete automation | 🔐 bearer |
| `GET` | `/automations/{automation_id}` | GET /v1/automations/{id} - Get automation detail | 🔐 bearer |
| `PUT` | `/automations/{automation_id}` | PUT /v1/automations/{id} - Update automation | 🔐 bearer |
| `PATCH` | `/automations/{automation_id}/enable` | PATCH /v1/automations/{id}/enable - Toggle enabled | 🔐 bearer |
| `POST` | `/automations/{automation_id}/run` | POST /v1/automations/{id}/run - Execute automation manually | 🔐 bearer |
| `POST` | `/automations/{automation_id}/test` | POST /v1/automations/{id}/test - Dry-run test | 🔐 bearer |

## Context

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `POST` | `/v1/context/clear` | POST /context/clear — Cancel the active manual mode override and revert to automatic context. | 🔐 bearer |
| `GET` | `/v1/context/current` | GET /context/current — Return the current contextual mode state. | 🔐 bearer |
| `GET` | `/v1/context/history` | GET /context/history — Return the chronological history of mode changes. | 🔐 bearer |
| `POST` | `/v1/context/override` | POST /context/override — Force a manual contextual mode override and record intelligence feedback. | 🔐 bearer |
| `GET` | `/v1/context/productivity` | GET /context/productivity — Return productivity metrics broken down by contextual mode. | 🔐 bearer |
| `GET` | `/v1/context/stats` | GET /context/stats — Return aggregated usage statistics per contextual mode. | 🔐 bearer |

## Decision

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/decision/agent-health` | Return health status for all registered agents. | 🔐 bearer |
| `GET` | `/decision/audit` | Retrieve the decision audit trail with optional query filters. | 🔐 bearer |
| `GET` | `/decision/config` | Return the current Decision Engine configuration. | 🔐 bearer |
| `POST` | `/decision/evaluate` | Evaluate an action through the Decision Engine. | 🔐 bearer |
| `GET` | `/decision/metrics` | Return Decision Engine metrics in Prometheus text format. | 🔐 bearer |
| `POST` | `/decision/override` | Create a new master override for the Decision Engine. | 🔐 bearer |
| `DELETE` | `/decision/override/{id}` | Revoke an active master override by ID. | 🔐 bearer |
| `GET` | `/decision/overrides/active` | List all currently active master overrides. | 🔐 bearer |
| `GET` | `/decision/stats` | Return aggregate Decision Engine statistics. | 🔐 bearer |
| `DELETE` | `/decision/validation/{id}` | Delete a specific validation request by ID. | 🔐 bearer |
| `POST` | `/decision/validation/{id}/resolve` | Approve or reject a pending validation and execute the associated action if approved. | 🔐 bearer |
| `DELETE` | `/decision/validations/expired` | Delete all expired validation requests. | 🔐 bearer |
| `GET` | `/decision/validations/expired` | List all expired validation requests. | 🔐 bearer |
| `GET` | `/decision/validations/pending` | List all pending validation requests. | 🔐 bearer |

## Environment

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/environment/sensors` | GET /v1/environment/sensors | 🔐 bearer |
| `GET` | `/environment/sensors/{sensor_id}` | GET /v1/environment/sensors/:sensor_id | 🔐 bearer |
| `GET` | `/environment/sensors/{sensor_id}/history` | GET /v1/environment/sensors/:sensor_id/history?hours=24 | 🔐 bearer |
| `GET` | `/environment/{room_id}` | GET /v1/environment/:room_id | 🔐 bearer |
| `GET` | `/environment/{room_id}/history` | GET /v1/environment/:room_id/history?hours=24 | 🔐 bearer |

## Intelligence

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/intelligence/config` | GET /v1/intelligence/config | 🔐 bearer |
| `PUT` | `/intelligence/config` | PUT /v1/intelligence/config | 🔐 bearer |
| `GET` | `/intelligence/features` | GET /v1/intelligence/features | 🔐 bearer |
| `POST` | `/intelligence/features` | POST /v1/intelligence/features | 🔐 bearer |
| `POST` | `/intelligence/feedback` | POST /v1/intelligence/feedback | 🔐 bearer |
| `GET` | `/intelligence/health` | GET /v1/intelligence/health | 🔐 bearer |
| `GET` | `/intelligence/patterns` | GET /v1/intelligence/patterns | 🔐 bearer |
| `GET` | `/intelligence/patterns/export` | GET /v1/intelligence/patterns/export | 🔐 bearer |
| `GET` | `/intelligence/prediction2` | GET /v1/intelligence/prediction2 | 🔐 bearer |
| `GET` | `/intelligence/predictions` | GET /v1/intelligence/predictions | 🔐 bearer |
| `GET` | `/intelligence/session` | GET /v1/intelligence/session | 🔐 bearer |
| `GET` | `/intelligence/shadow-stats` | GET /v1/intelligence/shadow-stats | 🔐 bearer |
| `GET` | `/intelligence/signals` | GET /v1/intelligence/signals | 🔐 bearer |
| `GET` | `/intelligence/status` | GET /v1/intelligence/status | 🔐 bearer |
| `GET` | `/intelligence/vector` | GET /v1/intelligence/vector | 🔐 bearer |

## Modes

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/v1/modes` | GET /modes - Liste tous les modes | 🔐 bearer |
| `POST` | `/v1/modes` | POST /modes - Crée un nouveau mode | 🔐 bearer |
| `DELETE` | `/v1/modes/{id}` | DELETE /modes/:id - Supprime un mode | 🔐 bearer |
| `GET` | `/v1/modes/{id}` | GET /modes/:id - Récupère un mode par ID | 🔐 bearer |
| `PUT` | `/v1/modes/{id}` | PUT /modes/:id - Met à jour un mode | 🔐 bearer |

## Notifications

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/notification-types` | GET /notification-types — List all notification type configurations. | 🔐 bearer |
| `GET` | `/notification-types/{type_id}` | GET /notification-types/{type_id} — Retrieve a specific notification type configuration. | 🔐 bearer |
| `PUT` | `/notification-types/{type_id}` | PUT /notification-types/{type_id} — Update a notification type configuration. | 🔐 bearer |
| `DELETE` | `/notifications` | DELETE /notifications — Delete all notifications at once. | 🔐 bearer |
| `GET` | `/notifications` | GET /notifications — List all notifications from history. | 🔐 bearer |
| `POST` | `/notifications` | POST /notifications — Send a new notification with optional priority and actions. | 🔐 bearer |
| `GET` | `/notifications/active` | GET /notifications/active — List all unacknowledged notifications. | 🔐 bearer |
| `GET` | `/notifications/tokens` | GET /notifications/tokens — List all registered FCM tokens. | 🔐 bearer |
| `POST` | `/notifications/tokens` | POST /notifications/tokens — Register an FCM push token for a user/device. | 🔐 bearer |
| `DELETE` | `/notifications/{id}` | DELETE /notifications/{id} — Delete a notification by ID. | 🔐 bearer |
| `POST` | `/notifications/{id}/acknowledge` | POST /notifications/{id}/acknowledge — Acknowledge a notification by ID. | 🔐 bearer |

## Schedule

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/v1/schedule` | GET /schedule - Récupère le planning complet | 🔐 bearer |
| `GET` | `/v1/schedule/current` | GET /schedule/current - Récupère le mode actif selon le planning | 🔐 bearer |
| `PUT` | `/v1/schedule/default` | PUT /schedule/default - Définit le mode par défaut | 🔐 bearer |
| `GET` | `/v1/schedule/rules` | GET /schedule/rules - Liste toutes les règles | 🔐 bearer |
| `POST` | `/v1/schedule/rules` | POST /schedule/rules - Crée une nouvelle règle | 🔐 bearer |
| `DELETE` | `/v1/schedule/rules/{id}` | DELETE /schedule/rules/:id - Supprime une règle | 🔐 bearer |
| `PUT` | `/v1/schedule/rules/{id}` | PUT /schedule/rules/:id - Met à jour une règle | 🔐 bearer |

## System

| Méthode | Chemin | Description | Auth |
|---------|--------|-------------|------|
| `GET` | `/contracts` | Return the list of all registered MQTT contract names. | 🔐 bearer |
| `GET` | `/contracts/{name}` | Return a single MQTT contract by name, or 404 if not found. | 🔐 bearer |
| `GET` | `/health/ready` | GET /health/ready — Readiness probe pour monitoring externe (healthcheck.io, UptimeRobot, k8s) | 🔓 public |
| `GET` | `/hosts` | Return a list of all known hosts with their current state. | 🔐 bearer |
| `GET` | `/hosts/{id}` | Return a single host by ID, or 404 if not found. | 🔐 bearer |
| `GET` | `/logs` | GET /logs - Récupère les logs kernel depuis journalctl | 🔐 bearer |
| `GET` | `/metrics` | GET /metrics - Prometheus scraping endpoint (public, no auth required) | 🔓 public |
| `GET` | `/system/health` | Return full infrastructure health status (MQTT, agents, plugins). | 🔓 public |
| `GET` | `/v1/metrics/agents` | Return per-agent telemetry (CPU, RAM, disk, network, processes). | 🔓 public |
| `GET` | `/v1/metrics/system` | Return aggregated kernel performance metrics (runtime, MQTT, agents, plugins, context, decisions). | 🔓 public |
| `POST` | `/wake` | Send a Wake-on-LAN magic packet to the specified host. | 🔐 bearer |
