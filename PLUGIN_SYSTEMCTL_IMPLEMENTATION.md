# Plugin Systemctl Control Routes - Implementation Summary

## Overview
Successfully added 4 new HTTP API routes to the Symbion kernel for managing plugins via systemd user services. This enables remote control of plugin lifecycle through the kernel's REST API.

---

## Changes Made

### File Modified
- **Path:** `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/http.rs`
- **Lines Added:** ~125 lines (routes + handlers + documentation)

### Routes Added (4 total)

#### 1. POST /v1/plugins/:name/start
- **Location:** Line 248
- **Handler:** `start_plugin_systemctl` (Line 1108)
- **Protection:** CSRF + JWT authentication
- **Function:** Executes `systemctl --user start symbion-plugin-{name}`

#### 2. POST /v1/plugins/:name/stop
- **Location:** Line 249
- **Handler:** `stop_plugin_systemctl` (Line 1139)
- **Protection:** CSRF + JWT authentication
- **Function:** Executes `systemctl --user stop symbion-plugin-{name}`

#### 3. POST /v1/plugins/:name/restart
- **Location:** Line 250
- **Handler:** `restart_plugin_systemctl` (Line 1169)
- **Protection:** CSRF + JWT authentication
- **Function:** Executes `systemctl --user restart symbion-plugin-{name}`

#### 4. GET /v1/plugins/:name/status
- **Location:** Line 273
- **Handler:** `get_plugin_systemctl_status` (Line 1199)
- **Protection:** JWT authentication only
- **Function:** Executes `systemctl --user is-active symbion-plugin-{name}`

---

## Handler Functions

All handlers are located in the **PLUGIN SYSTEMCTL ENDPOINTS** section starting at line 1105.

### Common Features Across All Handlers:

1. **Input Validation**
   - Plugin name must contain only alphanumeric characters and hyphens
   - Returns `400 BAD_REQUEST` for invalid names
   - Prevents command injection attacks

2. **Service Name Formatting**
   - Automatically prefixes plugin name with `symbion-plugin-`
   - Example: `notes` → `symbion-plugin-notes`

3. **Error Handling**
   - Logs stderr output on systemctl failures
   - Returns `500 INTERNAL_SERVER_ERROR` on command execution errors
   - Uses `tokio::process::Command` for async execution

4. **Response Format**
   - Success responses include: status, message, service name
   - Status endpoint returns: service, status string, is_active boolean

### Handler Signatures

```rust
async fn start_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode>

async fn stop_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode>

async fn restart_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode>

async fn get_plugin_systemctl_status(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode>
```

---

## Security Architecture

### Protection Layers

1. **JWT Authentication (All Routes)**
   - All 4 routes require valid JWT token
   - Applied via `require_auth` middleware

2. **CSRF Protection (POST Routes)**
   - Start, stop, and restart require CSRF token
   - GET status route only needs JWT (safe operation)
   - Applied via `require_csrf` middleware

3. **Input Validation**
   - Alphanumeric + hyphen validation prevents command injection
   - Service name construction is safe (no user input concatenation)

### Route Placement

- **CSRF-protected routes** (lines 247-250): Placed with other destructive operations (agent shutdown, reboot, hibernate)
- **API routes** (line 273): Status check with read-only agent endpoints

---

## Testing Guide

### Prerequisites
1. Kernel running on `http://localhost:8080` or `https://localhost:8443`
2. Valid JWT token
3. Plugin systemd service installed (e.g., `symbion-plugin-notes.service`)

### Step 1: Obtain JWT Token
```bash
# Login and extract JWT
JWT=$(curl -s -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your_password"}' | jq -r '.token')
```

### Step 2: Get CSRF Token
```bash
CSRF=$(curl -s http://localhost:8080/auth/csrf/nonce \
  -H "Authorization: Bearer $JWT" | jq -r '.nonce')
```

### Step 3: Check Plugin Status
```bash
curl http://localhost:8080/v1/plugins/notes/status \
  -H "Authorization: Bearer $JWT"
```

**Expected Response:**
```json
{
  "service": "symbion-plugin-notes",
  "status": "active",
  "is_active": true
}
```

### Step 4: Start Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/start \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF"
```

**Expected Response:**
```json
{
  "status": "success",
  "message": "Plugin notes started",
  "service": "symbion-plugin-notes"
}
```

### Step 5: Restart Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/restart \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF"
```

### Step 6: Stop Plugin
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/stop \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF"
```

### Error Cases to Test

1. **Invalid Plugin Name**
```bash
curl -X POST http://localhost:8080/v1/plugins/test@invalid/start \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF"
# Expected: 400 BAD_REQUEST
```

2. **Non-existent Service**
```bash
curl http://localhost:8080/v1/plugins/nonexistent/status \
  -H "Authorization: Bearer $JWT"
# Expected: 200 OK with status="inactive" and is_active=false
```

3. **Missing Authentication**
```bash
curl http://localhost:8080/v1/plugins/notes/status
# Expected: 401 UNAUTHORIZED
```

4. **Missing CSRF Token**
```bash
curl -X POST http://localhost:8080/v1/plugins/notes/start \
  -H "Authorization: Bearer $JWT"
# Expected: 403 FORBIDDEN
```

---

## Integration with PWA Dashboard

The PWA dashboard can now add plugin control buttons:

```javascript
// Example: Add start/stop buttons in plugins-widget.js

async function startPlugin(pluginName) {
  const token = localStorage.getItem('jwt');
  const csrf = await fetchCsrfToken();

  const response = await fetch(`/v1/plugins/${pluginName}/start`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'X-CSRF-Token': csrf
    }
  });

  return response.json();
}

async function stopPlugin(pluginName) {
  const token = localStorage.getItem('jwt');
  const csrf = await fetchCsrfToken();

  const response = await fetch(`/v1/plugins/${pluginName}/stop`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'X-CSRF-Token': csrf
    }
  });

  return response.json();
}

async function getPluginStatus(pluginName) {
  const token = localStorage.getItem('jwt');

  const response = await fetch(`/v1/plugins/${pluginName}/status`, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });

  return response.json();
}
```

---

## Systemd Service Requirements

For the routes to work, plugins must be installed as systemd user services:

### Service File Location
`~/.config/systemd/user/symbion-plugin-{name}.service`

### Example Service File
```ini
[Unit]
Description=Symbion Plugin - Notes
After=network.target

[Service]
Type=simple
ExecStart=/home/user/symbion-plugin-notes/target/release/symbion-plugin-notes
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

### Enable and Start
```bash
systemctl --user enable symbion-plugin-notes
systemctl --user start symbion-plugin-notes
systemctl --user status symbion-plugin-notes
```

---

## Code Architecture Compliance

### Follows Existing Patterns

1. **Route Organization**
   - CSRF-protected routes grouped with agent control routes
   - Read-only routes in api_routes section
   - Consistent with existing structure

2. **Handler Style**
   - Uses `Path` extractor for URL parameters
   - Uses `State` for application state (though not used here)
   - Returns `Result<Json<...>, StatusCode>`
   - Logs errors with `eprintln!` prefix `[kernel]`

3. **Security Consistency**
   - Same middleware stack as agent control endpoints
   - CSRF protection for state-changing operations
   - JWT authentication for all routes

4. **Error Handling**
   - Returns appropriate HTTP status codes
   - Logs detailed error information
   - Returns JSON responses

---

## Future Enhancements

### Potential Additions

1. **Enhanced Status Information**
   - Parse `systemctl status` output for detailed info
   - Include uptime, memory usage, recent logs

2. **Bulk Operations**
   - POST /v1/plugins/start-all
   - POST /v1/plugins/stop-all
   - POST /v1/plugins/restart-all

3. **Service Management**
   - POST /v1/plugins/:name/enable (systemctl enable)
   - POST /v1/plugins/:name/disable (systemctl disable)

4. **Log Access**
   - GET /v1/plugins/:name/logs (journalctl -u symbion-plugin-{name})

5. **Health Checks**
   - Integrate with existing health monitoring
   - Add plugin status to /system/health endpoint

---

## Related Files

### Documentation
- Test guide: `/home/eridwyn/RustroverProjects/NewSymbion/TEST_PLUGIN_SYSTEMCTL_ROUTES.md`
- This implementation summary: `/home/eridwyn/RustroverProjects/NewSymbion/PLUGIN_SYSTEMCTL_IMPLEMENTATION.md`

### Code
- HTTP routes and handlers: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/http.rs`

### Related Systems
- Plugin registry: `symbion-kernel/src/plugin_proxy.rs`
- Authentication: `symbion-kernel/src/auth.rs`
- CSRF protection: `symbion-kernel/src/csrf.rs`

---

## Summary

Successfully implemented 4 new API endpoints for systemctl-based plugin lifecycle management:

- **Routes Added:** 4 (start, stop, restart, status)
- **Handlers Added:** 4 async functions
- **Security:** CSRF + JWT protection
- **Input Validation:** Alphanumeric + hyphen only
- **Service Format:** `symbion-plugin-{name}`
- **Integration:** Ready for PWA dashboard controls

The implementation follows existing kernel patterns, maintains security standards, and provides a clean API for remote plugin management via systemd.
