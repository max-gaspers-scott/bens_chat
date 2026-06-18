-- =============================================================================
-- Migration 0002: Transform old schema → new schema
--
-- Old schema summary:
--   users        (user_id UUID PK, username, email, password_hash, phone_number, created_at)
--   chats        (chat_id UUID PK, chat_name, created_at)
--   user_chats   (user_id, chat_id)  -- many-to-many join
--   messages     (message_id UUID PK, chat_id, sender_id, content JSONB, sent_at, minio_url)
--
-- New schema (already created by 0001_data.sql):
--   users            (name VARCHAR PK, phone_number, email, passwrod_hash)
--   notes            (note_id, text, refers_to_user_name, created_by_user_name, contact_name)
--   messages         (sent_at, message_id, sender_name, parent UUID, content JSONB)
--   chats            (chat_id, root_message_id)
--   chat_participants (chat_participant_id, chat_id, user_name)
--
-- Mapping decisions:
--   • users.username         → users.name  (PK; all usernames are already unique)
--   • old chats              → one root message per chat (parent=NULL, content={"title": chat_name})
--                             + one chats row pointing to that root message
--   • old messages           → messages (parent = root_message_id of their chat)
--                             content = {"text": <old content>, "url": <minio_url>}  (url omitted if NULL)
--   • user_chats             → chat_participants
-- =============================================================================

BEGIN;

-- ---------------------------------------------------------------------------
-- STEP 1: Rename old tables so new ones (from 0001) can coexist during migration
-- ---------------------------------------------------------------------------

ALTER TABLE IF EXISTS messages    RENAME TO _old_messages;
ALTER TABLE IF EXISTS chats       RENAME TO _old_chats;
ALTER TABLE IF EXISTS user_chats  RENAME TO _old_user_chats;
ALTER TABLE IF EXISTS users       RENAME TO _old_users;

-- Drop the uuid-ossp extension dependency — gen_random_uuid() (pgcrypto / built-in)
-- is used in the new schema instead. We keep the extension for now and drop at end.

-- ---------------------------------------------------------------------------
-- STEP 2: Re-create the new schema tables (idempotent — already exist from 0001,
--         but if this migration runs on a fresh DB they need to exist first)
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS users (
    name          VARCHAR(255) PRIMARY KEY,
    phone_number  VARCHAR(255),
    email         VARCHAR(255) UNIQUE,
    passwrod_hash TEXT NOT NULL          -- intentional spelling matches target schema
);

CREATE TABLE IF NOT EXISTS notes (
    note_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    text                 TEXT NOT NULL,
    refers_to_user_name  VARCHAR(255) REFERENCES users(name),
    created_by_user_name VARCHAR(255) NOT NULL REFERENCES users(name),
    contact_name         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    sent_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_name VARCHAR(255) NOT NULL REFERENCES users(name),
    parent      UUID REFERENCES messages(message_id),
    content     JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS chats (
    chat_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    root_message_id UUID UNIQUE NOT NULL REFERENCES messages(message_id)
);

CREATE TABLE IF NOT EXISTS chat_participants (
    chat_participant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id             UUID NOT NULL REFERENCES chats(chat_id),
    user_name           VARCHAR(255) NOT NULL REFERENCES users(name),
    UNIQUE (chat_id, user_name)
);

-- ---------------------------------------------------------------------------
-- STEP 3: Migrate users
--   username → name (new PK)
--   password_hash → passwrod_hash (target schema typo preserved)
--   email, phone_number carried over; created_at dropped
-- ---------------------------------------------------------------------------

INSERT INTO users (name, phone_number, email, passwrod_hash)
SELECT
    username,
    phone_number,
    email,
    password_hash
FROM _old_users
ON CONFLICT (name) DO NOTHING;  -- safety: skip if somehow already present

-- ---------------------------------------------------------------------------
-- STEP 4: Create a root message for each old chat
--
--   We need a stable mapping: old chat_id → new root message_id.
--   We use a temporary table to hold this mapping.
--
--   Root message content: {"title": "<chat_name>"}
--   sender_name: we pick the earliest member of that chat; every chat must
--   have at least one member for the NOT NULL constraint to be satisfied.
--   If a chat has NO members we fall back to the first user alphabetically.
-- ---------------------------------------------------------------------------

CREATE TEMP TABLE _chat_to_root_msg (
    old_chat_id     UUID PRIMARY KEY,
    new_message_id  UUID NOT NULL DEFAULT gen_random_uuid()
);

INSERT INTO _chat_to_root_msg (old_chat_id)
SELECT chat_id FROM _old_chats;

-- Insert root messages (one per old chat)
INSERT INTO messages (message_id, sent_at, sender_name, parent, content)
SELECT
    m.new_message_id,
    COALESCE(oc.created_at, NOW()),
    -- Pick the username of the first user who joined this chat, or fallback
    COALESCE(
        (
            SELECT u.username
            FROM _old_user_chats uc
            JOIN _old_users u ON u.user_id = uc.user_id
            WHERE uc.chat_id = oc.chat_id
            ORDER BY uc.joined_at ASC
            LIMIT 1
        ),
        (SELECT username FROM _old_users ORDER BY username ASC LIMIT 1)
    ),
    NULL,  -- root message: no parent
    jsonb_build_object(
        'title', COALESCE(oc.chat_name, 'Untitled Chat')
    )
FROM _old_chats oc
JOIN _chat_to_root_msg m ON m.old_chat_id = oc.chat_id;

-- ---------------------------------------------------------------------------
-- STEP 5: Populate new chats table
--   Each old chat → one new chats row pointing to its root message
-- ---------------------------------------------------------------------------

INSERT INTO chats (chat_id, root_message_id)
SELECT
    oc.chat_id,          -- reuse the same UUID for continuity
    m.new_message_id
FROM _old_chats oc
JOIN _chat_to_root_msg m ON m.old_chat_id = oc.chat_id;

-- ---------------------------------------------------------------------------
-- STEP 6: Migrate old messages
--
--   • parent = root_message_id of their chat  (they become direct children of root)
--   • sender_name = username of sender (messages whose sender was SET NULL are
--     skipped — the new schema requires sender_name NOT NULL)
--   • content: merge old content JSONB with minio_url if present
--       → {"text": <old-content>, "url": "<minio_url>"}   when minio_url IS NOT NULL
--       → {"text": <old-content>}                         when minio_url IS NULL
-- ---------------------------------------------------------------------------

INSERT INTO messages (message_id, sent_at, sender_name, parent, content)
SELECT
    om.message_id,
    om.sent_at,
    u.username,           -- resolved via sender_id → _old_users
    m.new_message_id,     -- parent = root message of the chat
    CASE
        WHEN om.minio_url IS NOT NULL THEN
            jsonb_build_object('text', om.content, 'url', om.minio_url)
        ELSE
            jsonb_build_object('text', om.content)
    END
FROM _old_messages om
JOIN _old_users u        ON u.user_id     = om.sender_id   -- excludes SET NULL senders
JOIN _chat_to_root_msg m ON m.old_chat_id = om.chat_id;

-- ---------------------------------------------------------------------------
-- STEP 7: Migrate user_chats → chat_participants
-- ---------------------------------------------------------------------------

INSERT INTO chat_participants (chat_id, user_name)
SELECT
    uc.chat_id,        -- same UUID reused in new chats table
    u.username
FROM _old_user_chats uc
JOIN _old_users u ON u.user_id = uc.user_id
ON CONFLICT (chat_id, user_name) DO NOTHING;

-- ---------------------------------------------------------------------------
-- STEP 8: Drop old tables and temporary helpers
-- ---------------------------------------------------------------------------

DROP TABLE _old_messages;
DROP TABLE _old_user_chats;
DROP TABLE _old_chats;
DROP TABLE _old_users;
DROP TABLE _chat_to_root_msg;

-- Optionally drop the uuid-ossp extension if nothing else uses it.
-- Comment out if other parts of your DB still rely on uuid_generate_v4().
DROP EXTENSION IF EXISTS "uuid-ossp";

COMMIT;
