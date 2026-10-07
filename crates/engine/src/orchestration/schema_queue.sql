CREATE TABLE IF NOT EXISTS orchestration_queue_lifecycle (
    thread_id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_queue_intents (
    thread_id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL
) STRICT;
-- Keep upload paths after the queue intent is consumed, until its steering
-- effect executes. A later intent sync must not erase a promoted attachment.
CREATE TABLE IF NOT EXISTS orchestration_queue_attachments (
    thread_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    paths_json TEXT NOT NULL,
    PRIMARY KEY(thread_id,message_id)
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_queue_patches (
    command_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    payload_json TEXT NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS orchestration_queue_patches_thread
    ON orchestration_queue_patches(thread_id,sequence);
-- Reserve user request identities before dispatch. A lost response/restart can
-- retry the exact command, but cannot silently change its target or content.
CREATE TABLE IF NOT EXISTS orchestration_queue_user_requests (
    command_id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_session_user_requests (
    command_id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL
) STRICT;
