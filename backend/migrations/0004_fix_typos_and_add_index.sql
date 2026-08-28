-- =============================================================================
-- Migration 0004: Fix column name typos and add parent_id index
--
-- Changes:
--   1. Rename users.passwrod_hash  → users.password_hash  (fix typo)
--   2. Rename messages.parent      → messages.parent_id   (add _id suffix for clarity)
--   3. Add index on messages.parent_id for faster threaded message lookups
-- =============================================================================

BEGIN;

-- 1. Fix the password_hash typo on the users table
ALTER TABLE users RENAME COLUMN passwrod_hash TO password_hash;

-- 2. Rename parent → parent_id on the messages table
ALTER TABLE messages RENAME COLUMN parent TO parent_id;

-- 3. Index for looking up child messages by parent
CREATE INDEX idx_messages_parent_id ON messages (parent_id);

COMMIT;
