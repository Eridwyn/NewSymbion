# CSRF Protection Implementation Guide

**Version**: 0.3.0-alpha.1
**Status**: ✅ Production Ready
**Date**: 30 October 2025

---

## 🎯 Overview

Symbion implements **single-use CSRF nonces** with:
- TTL: 5 minutes
- One-time use (invalidated after first request)
- Required on all destructive operations (POST/PUT/DELETE)

---

## 🔒 CSRF Attack Prevention

### What is CSRF?

**Cross-Site Request Forgery**: Malicious site tricks browser into sending authenticated request to your API.

**Example Attack** (without CSRF protection):
```html
<!-- On evil.com -->
<form action="https://symbion.local:8443/agents/shutdown" method="POST">
  <input type="hidden" name="agent_id" value="eridwyn-Salon">
</form>
<script>document.forms[0].submit();</script>
```

**Result**: If user is logged in, browser sends JWT cookie → agent shuts down.

**CSRF Protection Blocks This**: Nonce required, malicious site cannot obtain it.

---

## 🔐 Implementation

### Nonce Generation

**Endpoint**: `GET /auth/csrf/nonce`

**Headers Required**:
```
Authorization: Bearer <jwt_token>
```

**Response (200)**:
```json
{
  "nonce": "550e8400-e29b-41d4-a716-446655440000",
  "expires_in_seconds": 300
}
```

**Implementation** (`symbion-kernel/src/csrf.rs:57-86`):
```rust
use uuid::Uuid;
use std::time::SystemTime;

pub fn generate_nonce(&self) -> String {
    Uuid::new_v4().to_string()
}

pub fn store_nonce(&self, nonce: String) {
    let mut store = self.nonces.write().unwrap();
    let expires_at = SystemTime::now() + Duration::from_secs(300);  // 5 min

    store.insert(nonce, expires_at);

    // Cleanup expired nonces
    store.retain(|_, &mut exp| exp > SystemTime::now());
}
```

### Nonce Validation

**Middleware** (`symbion-kernel/src/http.rs:117-157`):
```rust
async fn require_csrf(
    State(app): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // 1. Extract X-CSRF-Token header
    let csrf_token = request.headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::FORBIDDEN, "Missing or invalid CSRF token".to_string()))?;

    // 2. Validate nonce (exists + not expired)
    let valid = app.csrf_store.validate(csrf_token);

    if !valid {
        return Err((StatusCode::FORBIDDEN, "CSRF token expired or already consumed".to_string()));
    }

    // 3. Invalidate nonce (one-time use)
    app.csrf_store.invalidate(csrf_token);

    Ok(next.run(request).await)
}
```

**Validation Logic** (`symbion-kernel/src/csrf.rs:88-120`):
```rust
pub fn validate(&self, nonce: &str) -> bool {
    let store = self.nonces.read().unwrap();

    match store.get(nonce) {
        Some(&expires_at) => {
            // Check if expired
            SystemTime::now() < expires_at
        },
        None => false,  // Nonce not found or already used
    }
}

pub fn invalidate(&self, nonce: &str) {
    let mut store = self.nonces.write().unwrap();
    store.remove(nonce);  // Single-use: remove after validation
}
```

---

## 🛡️ Protected Endpoints

### Routes Requiring CSRF

**All POST/PUT/DELETE routes under `/v1/*`**:

**Agent Control**:
- `POST /v1/agents/{id}/shutdown`
- `POST /v1/agents/{id}/reboot`
- `POST /v1/agents/{id}/hibernate`
- `POST /v1/agents/{id}/kill-process`

**Context Control**:
- `POST /v1/context/override`
- `POST /v1/context/clear`

**Plugin Control**:
- `POST /v1/plugins/{name}/start`
- `POST /v1/plugins/{name}/stop`
- `POST /v1/plugins/{name}/restart`

**Notes Management**:
- `PUT /v1/ports/memo/{id}`
- `DELETE /v1/ports/memo/{id}`

### Routes NOT Requiring CSRF

**Public Endpoints**:
- `GET /health`
- `GET /system/health`
- `POST /auth/login`
- `GET /ca-certificate`

**Read-Only Authenticated Endpoints**:
- `GET /v1/agents`
- `GET /v1/agents/{id}`
- `GET /v1/context/current`
- `GET /v1/ports/memo`
- `GET /auth/csrf/nonce` (generates nonce, doesn't consume)

---

## 🔄 Usage Workflow

### Client-Side Flow

**1. Obtain JWT Token**:
```bash
TOKEN=$(curl -k -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test"}' \
  -s | jq -r '.token')
```

**2. Get CSRF Nonce**:
```bash
NONCE=$(curl -k -X GET https://localhost:8443/auth/csrf/nonce \
  -H "Authorization: Bearer $TOKEN" \
  -s | jq -r '.nonce')
```

**3. Execute Protected Action**:
```bash
curl -k -X POST https://localhost:8443/v1/context/override \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE" \
  -H "Content-Type: application/json" \
  -d '{"mode":"intime","duration_minutes":60}'
```

**4. Nonce Now Invalid** (one-time use):
```bash
# Same request with same nonce → 403 Forbidden
curl -k -X POST https://localhost:8443/v1/context/override \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE" \  # Already consumed
  -H "Content-Type: application/json" \
  -d '{"mode":"cravate","duration_minutes":120}'

# Response
{
  "error": "CSRF token expired or already consumed"
}
```

**5. Get New Nonce for Next Action**:
```bash
NONCE=$(curl -k -X GET https://localhost:8443/auth/csrf/nonce \
  -H "Authorization: Bearer $TOKEN" \
  -s | jq -r '.nonce')
```

---

## 🧩 Frontend Integration (PWA)

### Nonce Management Service

**File**: `pwa-dashboard/src/services/csrf-service.js`

```javascript
class CsrfService {
  constructor() {
    this.nonce = null;
    this.expiresAt = null;
  }

  async getNonce(jwtToken) {
    // Refresh if expired or missing
    if (!this.nonce || Date.now() / 1000 > this.expiresAt - 30) {
      await this.refreshNonce(jwtToken);
    }
    return this.nonce;
  }

  async refreshNonce(jwtToken) {
    const response = await fetch('https://localhost:8443/auth/csrf/nonce', {
      headers: {
        'Authorization': `Bearer ${jwtToken}`
      }
    });

    const data = await response.json();
    this.nonce = data.nonce;
    this.expiresAt = Date.now() / 1000 + data.expires_in_seconds;
  }

  invalidate() {
    this.nonce = null;  // Force refresh on next request
  }
}

export const csrfService = new CsrfService();
```

### Protected Action Example

```javascript
import { csrfService } from './services/csrf-service.js';

async function shutdownAgent(agentId) {
  const jwtToken = localStorage.getItem('jwt_token');
  const csrfNonce = await csrfService.getNonce(jwtToken);

  const response = await fetch(`https://localhost:8443/v1/agents/${agentId}/shutdown`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${jwtToken}`,
      'X-CSRF-Token': csrfNonce,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ reason: 'User requested shutdown' })
  });

  // Invalidate nonce after use (even if request failed)
  csrfService.invalidate();

  if (!response.ok) {
    throw new Error('Shutdown failed');
  }

  return response.json();
}
```

---

## ⚠️ Error Handling

### Missing CSRF Token

**HTTP 403 Forbidden**:
```json
{
  "error": "Missing or invalid CSRF token"
}
```

**Cause**: `X-CSRF-Token` header not present

### Expired/Consumed Nonce

**HTTP 403 Forbidden**:
```json
{
  "error": "CSRF token expired or already consumed"
}
```

**Causes**:
- Nonce older than 5 minutes
- Nonce already used in previous request
- Nonce never existed (invalid UUID)

### Invalid JWT Token

**HTTP 401 Unauthorized**:
```json
{
  "error": "Invalid or expired JWT token"
}
```

**Cause**: JWT validation failed (expired, wrong signature, missing)

---

## 🔒 Security Best Practices

### DO

✅ **Generate new nonce for each protected action**
✅ **Store nonces server-side** (never trust client-side storage)
✅ **Use short TTL** (5 minutes max)
✅ **Invalidate after use** (single-use tokens)
✅ **Validate JWT before issuing nonce**
✅ **Use cryptographically random UUIDs** (uuid v4)

### DON'T

❌ **Never include nonce in URL** (only headers)
❌ **Don't reuse nonces** (even if not expired)
❌ **Don't extend nonce TTL** on validation
❌ **Don't skip CSRF on "low-risk" endpoints** (attack surface)
❌ **Don't store nonces in cookies** (defeats CSRF purpose)

---

## 🧪 Testing

### Manual CSRF Test

**Test 1: Valid Nonce**:
```bash
TOKEN=$(curl -k -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}' -s | jq -r '.token')

NONCE=$(curl -k -X GET https://localhost:8443/auth/csrf/nonce \
  -H "Authorization: Bearer $TOKEN" -s | jq -r '.nonce')

# Should succeed (200 OK)
curl -k -X POST https://localhost:8443/v1/context/clear \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE"
```

**Test 2: Replay Attack** (should fail):
```bash
# Try using same nonce again
curl -k -X POST https://localhost:8443/v1/context/clear \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE"

# Expected: HTTP 403 Forbidden
# {"error":"CSRF token expired or already consumed"}
```

**Test 3: Missing Nonce** (should fail):
```bash
curl -k -X POST https://localhost:8443/v1/context/clear \
  -H "Authorization: Bearer $TOKEN"

# Expected: HTTP 403 Forbidden
# {"error":"Missing or invalid CSRF token"}
```

**Test 4: Expired Nonce** (should fail):
```bash
NONCE=$(curl -k -X GET https://localhost:8443/auth/csrf/nonce \
  -H "Authorization: Bearer $TOKEN" -s | jq -r '.nonce')

# Wait 6 minutes
sleep 360

# Try using expired nonce
curl -k -X POST https://localhost:8443/v1/context/clear \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE"

# Expected: HTTP 403 Forbidden
# {"error":"CSRF token expired or already consumed"}
```

---

## 📚 References

- **OWASP CSRF Prevention**: [Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- **Double Submit Cookie Pattern**: [OWASP Foundation](https://owasp.org/www-community/attacks/csrf)
- **Synchronizer Token Pattern**: [Spring Security Docs](https://docs.spring.io/spring-security/reference/features/exploits/csrf.html)

---

**Implementation Files**:
- `symbion-kernel/src/csrf.rs` - CSRF store and validation (287 lines)
- `symbion-kernel/src/http.rs` - require_csrf middleware (lines 117-157)
- `pwa-dashboard/src/services/csrf-service.js` - Frontend nonce management

**Related Documentation**:
- `security/implementation/AUTHENTICATION.md` - JWT token management
- `security/audits/SECURITY_AUDIT_2025-11-12.md` - CSRF security verification
