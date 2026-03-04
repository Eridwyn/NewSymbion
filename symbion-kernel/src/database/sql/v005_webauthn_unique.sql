-- v005: Fix WebAuthn credential duplication
-- Add UNIQUE constraint on credential_id and deduplicate existing rows

-- Step 1: Remove duplicate credentials, keeping the one with the latest last_used_at
DELETE FROM webauthn_credentials
WHERE id NOT IN (
    SELECT MIN(id)
    FROM webauthn_credentials
    GROUP BY credential_id
);

-- Step 2: Add UNIQUE index on credential_id to prevent future duplicates
CREATE UNIQUE INDEX IF NOT EXISTS idx_wa_credential_id_unique
    ON webauthn_credentials(credential_id);
