

-- 1. Add the column (root-first: [root, ..., grandparent, parent])
ALTER TABLE messages
    ADD COLUMN ancestors UUID[] NOT NULL DEFAULT '{}';

-- 2. Backfill existing rows, walking down from the true roots
WITH RECURSIVE tree AS (
    SELECT message_id, ARRAY[]::UUID[] AS ancestors
    FROM messages
    WHERE parent_id IS NULL

    UNION ALL

    SELECT m.message_id, t.ancestors || t.message_id
    FROM messages m
    JOIN tree t ON m.parent_id = t.message_id
)
UPDATE messages m
SET ancestors = t.ancestors
FROM tree t
WHERE m.message_id = t.message_id;



-- 3. Index for "everything under X" lookups
CREATE INDEX idx_messages_ancestors ON messages USING GIN (ancestors);

-- 4. Fill ancestors automatically on every new insert
CREATE FUNCTION set_message_ancestors() RETURNS trigger AS $$
BEGIN
    IF NEW.parent_id IS NULL THEN
        NEW.ancestors := '{}';
    ELSE
        SELECT p.ancestors || p.message_id
        INTO NEW.ancestors
        FROM messages p
        WHERE p.message_id = NEW.parent_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER messages_set_ancestors
    BEFORE INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION set_message_ancestors();

