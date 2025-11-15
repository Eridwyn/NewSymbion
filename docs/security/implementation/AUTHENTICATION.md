# Authentication Implementation Guide

**Version**: 0.3.0-alpha.1
**Status**: ✅ Production Ready
**Date**: 14 November 2025

---

## 🎯 Overview

Symbion implements **multi-factor JWT-based authentication** with:
- JWT tokens (HS256 signing)
- Bcrypt password hashing (cost factor 12)
- TOTP/MFA support (RFC 6238)
- Rate limiting (5 attempts / 15 min)

---

## 🔐 JWT Authentication

### Token Structure

**Algorithm**: HS256 (HMAC-SHA256)
**Expiry**: 8 hours (configurable via `SYMBION_TOKEN_EXPIRY_HOURS`)

**Claims**:
```json
{
  "sub": "username",
  "role": "admin|user|guest",
  "iat": 1730296800,
  "exp": 1730325600,
  "mfa_verified": true
}
```

### Login Flow

**Endpoint**: `POST /auth/login`

**Request**:
```json
{
  "username": "Mark",
  "password": "your_password",
  "totp_code": "123456"  // Optional - required if MFA enabled
}
```

**Response Success (200)**:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "username": "Mark",
  "role": "admin",
  "expires_at": 1730304000,
  "mfa_enabled": false
}
```

**Response Error - MFA Required (401)**:
```json
{
  "error": "MFA required",
  "mfa_required": true
}
```

**Response Error - Rate Limit (401)**:
```json
{
  "error": "Too many login attempts. Please wait 12 minute(s) before trying again."
}
```

### Token Usage

**All authenticated endpoints require**:
```
Authorization: Bearer <jwt_token>
```

**Example**:
```bash
curl -X GET https://localhost:8443/v1/agents \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Qi..." \
  -k
```

### JWT Secret Configuration

**Environment Variable**: `SYMBION_JWT_SECRET`

**Requirements**:
- Minimum 64 bytes (128 hex characters)
- Cryptographically secure random (use `openssl rand -hex 64`)
- Rotate every 90 days

**Generation**:
```bash
openssl rand -hex 64
# Output: ccc4e7528149d88f89e6c7b4190723b5aa7297c471938d0bee341e780b1929d0830a421192f048d01d26ec78dee98cca99c99b987ef879d3d162b8fa452263d4
```

**Configuration** (`.env`):
```bash
SYMBION_JWT_SECRET=ccc4e7528149d88f89e6c7b4190723b5aa7297c471938d0bee341e780b1929d0830a421192f048d01d26ec78dee98cca99c99b987ef879d3d162b8fa452263d4
SYMBION_TOKEN_EXPIRY_HOURS=8
```

---

## 🔑 Password Hashing (Bcrypt)

### Configuration

**Cost Factor**: 12 (VULN-005 fix)
**Library**: `bcrypt` crate

**Implementation** (`symbion-kernel/src/auth.rs:92-110`):
```rust
use bcrypt::{hash, verify, DEFAULT_COST};

const BCRYPT_COST: u32 = 12;  // Hardened from 10 to 12

pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, BCRYPT_COST)
        .map_err(|e| anyhow!("Failed to hash password: {}", e))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    bcrypt::verify(password, hash)
        .map_err(|e| anyhow!("Failed to verify password: {}", e))
}
```

### Password Requirements

| Requirement | Value |
|-------------|-------|
| Minimum length | 8 characters |
| Maximum length | 128 characters |
| Complexity | Not enforced (bcrypt cost compensates) |

**Recommendation**: Encourage strong passwords (16+ chars, mixed case, numbers, symbols)

### User Creation

**Default users** (`users.json`):
```json
{
  "users": [
    {
      "username": "Mark",
      "password_hash": "$2b$12$...",
      "role": "admin",
      "mfa_enabled": false
    }
  ]
}
```

**Password hash generation**:
```bash
# Using bcrypt CLI tool
htpasswd -nbBC 12 "" "your_password" | tr -d ':\n' | sed 's/$2y/$2b/'
```

---

## 🔐 MFA/TOTP Support

### Overview

**Standard**: RFC 6238 (Time-based One-Time Password)
**Compatible with**: Google Authenticator, Microsoft Authenticator, Authy, 1Password

### Setup Flow

**1. Initiate MFA Setup**

**Endpoint**: `POST /v1/auth/mfa/setup`

**Headers**:
```
Authorization: Bearer <jwt_token>
```

**Response (200)**:
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL...",
  "backup_codes": [
    "a1b2-c3d4-e5f6",
    "g7h8-i9j0-k1l2",
    "m3n4-o5p6-q7r8",
    "s9t0-u1v2-w3x4",
    "y5z6-a7b8-c9d0"
  ],
  "app_name": "Symbion",
  "username": "Mark"
}
```

**2. Scan QR Code**

User scans QR code with authenticator app.

**3. Verify TOTP Code**

**Endpoint**: `POST /v1/auth/mfa/verify`

**Request**:
```json
{
  "code": "123456"  // 6-digit TOTP code
}
```

**Response Success (200)**:
```json
{
  "success": true,
  "message": "MFA enabled successfully"
}
```

**Response Error (401)**:
```json
{
  "success": false,
  "error": "Invalid TOTP code"
}
```

**4. MFA Active**

All future logins require `totp_code` field.

### MFA Status Check

**Endpoint**: `GET /v1/auth/mfa/status`

**Response**:
```json
{
  "enabled": true,
  "username": "Mark"
}
```

### Disable MFA

**Endpoint**: `POST /v1/auth/mfa/disable`

**Response**:
```json
{
  "success": true,
  "message": "MFA disabled successfully"
}
```

**Notes**:
- Removes TOTP secret and backup codes
- Future logins no longer require MFA

---

## 🚦 Rate Limiting

### Protection Mechanism

**Location**: `symbion-kernel/src/auth.rs:145-171`

**Configuration**:
```rust
const MAX_LOGIN_ATTEMPTS: usize = 5;
const RATE_LIMIT_WINDOW_SECS: i64 = 900;  // 15 minutes
```

**Scope**: Per username (not IP-based)

### Implementation

```rust
fn check_rate_limit(&self, username: &str) -> Result<()> {
    let mut attempts = self.login_attempts.write();
    let now = OffsetDateTime::now_utc().unix_timestamp();

    let user_attempts = attempts.entry(username.to_string()).or_insert_with(Vec::new);

    // Remove expired attempts (> 15 min)
    user_attempts.retain(|&timestamp| now - timestamp < RATE_LIMIT_WINDOW_SECS);

    // Block if limit reached
    if user_attempts.len() >= MAX_LOGIN_ATTEMPTS {
        let oldest_attempt = user_attempts[0];
        let wait_time = RATE_LIMIT_WINDOW_SECS - (now - oldest_attempt);
        let wait_minutes = (wait_time / 60) + 1;

        anyhow::bail!("Too many login attempts. Please wait {} minute(s)", wait_minutes);
    }

    // Record new attempt
    user_attempts.push(now);

    Ok(())
}
```

### Behavior

| Scenario | Response |
|----------|----------|
| ≤ 5 attempts in 15 min | Login processed |
| \> 5 attempts in 15 min | HTTP 401 with wait time message |
| After 15 min window | Counter resets automatically |

**Note**: Counter stored in-memory (resets on kernel restart)

---

## 🔒 Security Best Practices

### JWT Secret Management

✅ **DO**:
- Generate with `openssl rand -hex 64`
- Store in `.env` file (never commit to git)
- Rotate every 90 days
- Use different secrets for dev/staging/prod

❌ **DON'T**:
- Hardcode in source code
- Use short/predictable secrets
- Share secrets across environments
- Store in version control

### Password Storage

✅ **DO**:
- Always use bcrypt with cost ≥ 12
- Never log passwords (even in debug mode)
- Validate minimum length (8+ chars)

❌ **DON'T**:
- Store plaintext passwords
- Use MD5/SHA1 for passwords
- Lower bcrypt cost below 12

### MFA Backup Codes

✅ **DO**:
- Generate 5+ backup codes
- Display only once at setup
- Invalidate after use
- Store hashed (like passwords)

❌ **DON'T**:
- Reuse backup codes
- Store plaintext codes
- Generate predictable codes

---

## 🧪 Testing

### Manual Login Test

```bash
# 1. Login without MFA
curl -k -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'

# 2. Extract token
TOKEN=$(curl -k -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}' \
  -s | jq -r '.token')

# 3. Use token
curl -k -X GET https://localhost:8443/v1/agents \
  -H "Authorization: Bearer $TOKEN"
```

### Rate Limit Test

```bash
# Attempt 6 logins in quick succession
for i in {1..6}; do
  curl -k -X POST https://localhost:8443/auth/login \
    -H "Content-Type: application/json" \
    -d '{"username":"test","password":"wrong"}' \
    -w "\nAttempt $i: %{http_code}\n"
  sleep 1
done

# Expected: First 5 return 401, 6th returns rate limit message
```

---

## 📚 References

- **JWT Spec**: [RFC 7519](https://tools.ietf.org/html/rfc7519)
- **TOTP Spec**: [RFC 6238](https://tools.ietf.org/html/rfc6238)
- **Bcrypt**: [OpenWall Crypt](https://www.openwall.com/crypt/)
- **OWASP Authentication**: [Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

---

**Implementation Files**:
- `symbion-kernel/src/auth.rs` - Core authentication logic
- `symbion-kernel/src/mfa.rs` - MFA/TOTP implementation (327 lines)
- `symbion-kernel/src/http.rs` - Login endpoint handlers

**Related Documentation**:
- `security/procedures/SECRETS_ROTATION_PROCEDURE.md` - Secret rotation guide
- `security/audits/SECURITY_AUDIT_2025-11-12.md` - Security audit (VULN-005)
