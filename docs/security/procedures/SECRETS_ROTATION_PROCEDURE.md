# Secrets Rotation Procedure

**Last rotation**: 14 November 2025
**Next rotation**: 12 February 2026 (90 days)

---

## 📋 Quick Start

```bash
# 1. Generate new secrets
NEW_JWT=$(openssl rand -hex 64)
NEW_API=$(openssl rand -hex 32)

# 2. Update .env file
cd /home/eridwyn/RustroverProjects/NewSymbion
sed -i "s/^SYMBION_JWT_SECRET=.*/SYMBION_JWT_SECRET=$NEW_JWT/" .env
sed -i "s/^SYMBION_API_KEY=.*/SYMBION_API_KEY=$NEW_API/" .env

# 3. Restart kernel and agents
pkill -f symbion-kernel
pkill -f symbion-agent-host
source .env && cargo run --release -p symbion-kernel &
cargo run --release -p symbion-agent-host &

# 4. Update monitoring scripts if needed
# scripts/monitor-symbion.sh uses same .env file (source ~/.env)
```

---

## 🔐 Secrets Inventory

### Current Secrets Locations

| Secret | Location | Format | Rotation Period |
|--------|----------|--------|-----------------|
| `SYMBION_JWT_SECRET` | `.env` | 128 hex (64 bytes) | 90 days |
| `SYMBION_API_KEY` | `.env` | 64 hex (32 bytes) | 90 days |

### Files Using Secrets

1. **symbion-kernel/src/auth.rs** - Reads `SYMBION_JWT_SECRET` via `std::env::var()`
2. **symbion-kernel/src/http.rs** - Reads `SYMBION_API_KEY` via `std::env::var()`
3. **All bash launch commands** - Must source `.env` or pass via environment

---

## ⚠️ Known Hardcoded Secrets (TO FIX)

**Location**: Various bash commands in development use test secrets:
- `SYMBION_API_KEY='s3cr3t-42'`
- `SYMBION_JWT_SECRET='test-secret-...'`

**Action required**:
1. Update launch scripts to source `.env` instead
2. Remove hardcoded secrets from bash commands
3. Document in setup guide to use `.env` only

---

## 🔄 Rotation Procedure (Detailed)

### Step 1: Generate New Secrets

```bash
# JWT Secret (64 bytes = 128 hex chars)
openssl rand -hex 64

# API Key (32 bytes = 64 hex chars)
openssl rand -hex 32
```

**Requirements**:
- JWT_SECRET: Minimum 64 bytes (128 hex chars)
- API_KEY: Minimum 32 bytes (64 hex chars)
- Use OpenSSL for cryptographically secure randomness

### Step 2: Backup Current Secrets

```bash
cp .env .env.backup-$(date +%Y%m%d)
chmod 600 .env.backup-*
```

### Step 3: Update .env File

Edit `.env` and replace:
- Line 6: `SYMBION_JWT_SECRET=<new_value>`
- Line 9: `SYMBION_API_KEY=<new_value>`

Update rotation date:
- Line 3: `# Last rotation: <current_date>`

### Step 4: Restart Services

**Kernel**:
```bash
# Find kernel process
ps aux | grep symbion-kernel

# Kill gracefully
pkill -SIGTERM -f symbion-kernel

# Wait for shutdown
sleep 2

# Start with new secrets
cd /home/eridwyn/RustroverProjects/NewSymbion
source .env
cargo run --release -p symbion-kernel
```

**Agents**:
```bash
# Kill agents
pkill -SIGTERM -f symbion-agent-host

# Restart
cargo run --release -p symbion-agent-host
```

### Step 5: Verify Authentication Works

```bash
# Test login with new JWT secret
curl -k -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'

# Should return HTTP 200 with token
```

### Step 6: Invalidate Old Secrets

1. Overwrite old secrets backup:
```bash
shred -vfz -n 3 .env.backup-*
```

2. Log rotation in security log:
```bash
echo "$(date): Secrets rotated successfully" >> /var/log/symbion-security.log
```

3. Update documentation:
```bash
# Update this file with new "Last rotation" date
# Update SECURITY_HARDENING_PHASE2.md if applicable
```

---

## 🚨 Emergency Rotation (Compromise)

If secrets are compromised, rotate immediately:

### Immediate Actions

1. **Kill all services**:
```bash
pkill -9 -f symbion-kernel
pkill -9 -f symbion-agent-host
```

2. **Generate new secrets** (as above)

3. **Invalidate all JWT tokens**:
   - Option 1: Restart kernel (stateless JWT = all invalidated)
   - Option 2: Implement token blacklist (future enhancement)

4. **Force re-authentication**:
   - All users must login again with new JWT secret
   - Update `users.json` if password hashes affected

5. **Audit logs**:
```bash
grep -i "authentication\|login" /tmp/kernel.log | tail -100
```

6. **Notify team** (if applicable)

---

## 📊 Rotation Schedule

| Secret Type | Period | Last Rotation | Next Due |
|-------------|--------|---------------|----------|
| JWT_SECRET | 90 days | 2025-11-14 | 2026-02-12 |
| API_KEY | 90 days | 2025-11-14 | 2026-02-12 |
| TLS Certificates | 365 days | 2025-11-11 | 2026-11-11 |

**Reminder**: Set calendar alert 7 days before next rotation.

---

## 📝 Post-Rotation Checklist

- [ ] New secrets generated with OpenSSL
- [ ] Old secrets backed up
- [ ] `.env` file updated
- [ ] Kernel restarted successfully
- [ ] Agents reconnected
- [ ] Authentication tested (login works)
- [ ] Monitoring scripts verified
- [ ] Documentation updated (rotation date)
- [ ] Old secrets securely deleted (shred)
- [ ] Next rotation scheduled (calendar)

---

## 🔗 References

- Security Audit: `docs/SECURITY_AUDIT_2025-11-12.md`
- Phase 2 Tracker: `docs/SECURITY_HARDENING_PHASE2.md`
- Auth Implementation: `symbion-kernel/src/auth.rs`

---

**Maintained by**: Mark
**Contact**: markchavatte@gmail.com
