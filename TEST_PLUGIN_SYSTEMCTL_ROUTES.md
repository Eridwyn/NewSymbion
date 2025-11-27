# Plugin Systemctl Routes - Test Guide

## Overview
Added 4 new HTTP API routes to the Symbion kernel for managing plugins via systemd user services.

## Routes Added

### 1. Start Plugin
**Endpoint:** `POST /v1/plugins/:name/start`
**Protection:** CSRF-protected (requires valid CSRF token)
**Authentication:** Requires JWT authentication

**Description:** Starts a plugin via `systemctl --user start symbion-plugin-{name}`

**Request:**
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/start \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-CSRF-Token: YOUR_CSRF_TOKEN"
```

**Response (Success):**
```json
{
  "status": "success",
  "message": "Plugin notes started",
  "service": "symbion-plugin-notes"
}
```

**Response (Error):**
- `400 BAD_REQUEST` - Invalid plugin name (must be alphanumeric + hyphens)
- `500 INTERNAL_SERVER_ERROR` - systemctl command failed

---

### 2. Stop Plugin
**Endpoint:** `POST /v1/plugins/:name/stop`
**Protection:** CSRF-protected (requires valid CSRF token)
**Authentication:** Requires JWT authentication

**Description:** Stops a plugin via `systemctl --user stop symbion-plugin-{name}`

**Request:**
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/stop \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-CSRF-Token: YOUR_CSRF_TOKEN"
```

**Response (Success):**
```json
{
  "status": "success",
  "message": "Plugin notes stopped",
  "service": "symbion-plugin-notes"
}
```

---

### 3. Restart Plugin
**Endpoint:** `POST /v1/plugins/:name/restart`
**Protection:** CSRF-protected (requires valid CSRF token)
**Authentication:** Requires JWT authentication

**Description:** Restarts a plugin via `systemctl --user restart symbion-plugin-{name}`

**Request:**
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/restart \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-CSRF-Token: YOUR_CSRF_TOKEN"
```

**Response (Success):**
```json
{
  "status": "success",
  "message": "Plugin notes restarted",
  "service": "symbion-plugin-notes"
}
```

---

### 4. Get Plugin Status
**Endpoint:** `GET /v1/plugins/:name/status`
**Protection:** JWT authentication only (no CSRF required for GET)
**Authentication:** Requires JWT authentication

**Description:** Gets plugin status via `systemctl --user is-active symbion-plugin-{name}`

**Request:**
```bash
curl http://localhost:8080/v1/plugins/notes/status \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

**Response (Active):**
```json
{
  "service": "symbion-plugin-notes",
  "status": "active",
  "is_active": true
}
```

**Response (Inactive):**
```json
{
  "service": "symbion-plugin-notes",
  "status": "inactive",
  "is_active": false
}
```

---

## Handler Functions Added

All handler functions are located in `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/http.rs` starting at line 1105.

### Functions:
1. `start_plugin_systemctl()` - Line 1108
2. `stop_plugin_systemctl()` - Line 1139
3. `restart_plugin_systemctl()` - Line 1169
4. `get_plugin_systemctl_status()` - Line 1199

### Security Features:
- **Input Validation:** Plugin name must be alphanumeric + hyphens only
- **Service Naming:** Auto-prefixes with `symbion-plugin-`
- **Error Logging:** Logs stderr output on failure
- **CSRF Protection:** Start/stop/restart require CSRF token
- **JWT Authentication:** All routes require valid JWT

---

## Testing Steps

### 1. Get CSRF Token
```bash
TOKEN=$(curl -s http://localhost:8080/auth/csrf/nonce -H "Authorization: Bearer $JWT" | jq -r '.nonce')
```

### 2. Check Plugin Status
```bash
curl http://localhost:8080/v1/plugins/notes/status \
  -H "Authorization: Bearer $JWT"
```

### 3. Start Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/start \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $TOKEN"
```

### 4. Restart Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/restart \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $TOKEN"
```

### 5. Stop Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/stop \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $TOKEN"
```

---

## Code Locations

### Route Definitions:
- **CSRF-protected routes (start/stop/restart):** Lines 247-250
- **API routes (status):** Line 273

### Handler Implementations:
- **Section header:** Line 1105
- **start_plugin_systemctl:** Lines 1108-1136
- **stop_plugin_systemctl:** Lines 1139-1166
- **restart_plugin_systemctl:** Lines 1169-1196
- **get_plugin_systemctl_status:** Lines 1199-1223

---

## Integration Notes

- All routes follow the existing kernel security model
- POST operations require CSRF protection (like agent shutdown/reboot)
- GET operation only requires JWT authentication
- Plugin names are validated to prevent command injection
- Service names are auto-formatted as `symbion-plugin-{name}`
- Stderr output is logged for debugging failed operations

---

## Example Plugin Services

The routes expect systemd user services named:
- `symbion-plugin-notes.service`
- `symbion-plugin-weather.service`
- `symbion-plugin-calendar.service`
- etc.

These services should be located in:
`~/.config/systemd/user/symbion-plugin-*.service`
