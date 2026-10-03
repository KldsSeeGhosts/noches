-- Ordered engine extension migration 1, on the profile DocsStore connection.
CREATE TABLE orchestration_host (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    host_id TEXT NOT NULL,
    epoch INTEGER NOT NULL
) STRICT;
CREATE TABLE orchestration_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    application_event_version INTEGER NOT NULL CHECK(application_event_version=2),
    event_id TEXT NOT NULL UNIQUE,
    command_id TEXT,
    aggregate_kind TEXT NOT NULL CHECK(aggregate_kind='thread'),
    stream_id TEXT NOT NULL,
    stream_version INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    envelope_json TEXT NOT NULL,
    UNIQUE(stream_id,stream_version)
) STRICT;
CREATE INDEX orchestration_events_command ON orchestration_events(command_id,sequence);
CREATE TABLE orchestration_command_receipts (
    command_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    command_type TEXT NOT NULL,
    accepted_at TEXT NOT NULL,
    result_sequence INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('accepted','rejected')),
    error TEXT
) STRICT;
CREATE TABLE orchestration_projection_metadata (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    schema_version INTEGER NOT NULL,
    last_sequence INTEGER NOT NULL
) STRICT;
INSERT INTO orchestration_projection_metadata VALUES(1,1,0);
CREATE TABLE orchestration_projection_threads (
    id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, run_id TEXT,
    status TEXT, provider_instance_id TEXT, provider_thread_id TEXT,
    provider_session_id TEXT, active_attempt_id TEXT, ordinal INTEGER,
    last_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE orchestration_projection_runs (
    id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, run_id TEXT,
    status TEXT NOT NULL, provider_instance_id TEXT NOT NULL, provider_thread_id TEXT,
    provider_session_id TEXT, active_attempt_id TEXT, ordinal INTEGER NOT NULL,
    last_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL,
    UNIQUE(thread_id,ordinal)
) STRICT;
CREATE INDEX orchestration_runs_owner ON orchestration_projection_runs(thread_id,status,active_attempt_id);
CREATE TABLE orchestration_projection_attempts (
    id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, run_id TEXT NOT NULL,
    status TEXT NOT NULL, provider_instance_id TEXT NOT NULL, provider_thread_id TEXT NOT NULL,
    provider_session_id TEXT, active_attempt_id TEXT, ordinal INTEGER NOT NULL,
    last_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL,
    UNIQUE(run_id,ordinal)
) STRICT;
CREATE TABLE orchestration_projection_nodes (
    id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, run_id TEXT,
    status TEXT NOT NULL, provider_instance_id TEXT, provider_thread_id TEXT,
    provider_session_id TEXT, active_attempt_id TEXT, ordinal INTEGER,
    last_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE orchestration_projection_records (
    kind TEXT NOT NULL, id TEXT NOT NULL, thread_id TEXT NOT NULL, run_id TEXT,
    status TEXT, provider_instance_id TEXT, provider_thread_id TEXT,
    provider_session_id TEXT, active_attempt_id TEXT, ordinal INTEGER,
    last_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL,
    PRIMARY KEY(kind,id,thread_id)
) STRICT;
CREATE INDEX orchestration_records_owner ON orchestration_projection_records(thread_id,kind,status);
CREATE TABLE orchestration_adoptions (
    legacy_chat_id TEXT PRIMARY KEY, thread_id TEXT NOT NULL UNIQUE,
    adopted_sequence INTEGER NOT NULL, provenance TEXT NOT NULL
) STRICT;
CREATE TABLE orchestration_effect_outbox (
    ordinal INTEGER PRIMARY KEY AUTOINCREMENT,
    effect_id TEXT NOT NULL UNIQUE,
    command_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    effect_type TEXT NOT NULL,
    lane TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    process_bound INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','running','succeeded','failed','cancelled','uncertain')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    available_at INTEGER NOT NULL,
    lease_owner TEXT,
    lease_expires_at INTEGER,
    dispatch_started INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    last_error TEXT
) STRICT;
CREATE INDEX orchestration_effect_lane ON orchestration_effect_outbox(thread_id,lane,ordinal,status);
CREATE TABLE orchestration_publication_batches (
    ordinal INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL UNIQUE,
    host_id TEXT NOT NULL,
    host_epoch INTEGER NOT NULL,
    through_sequence INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','published')),
    payload_json TEXT NOT NULL
) STRICT;
CREATE TABLE orchestration_publication_acks (
    batch_id TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    PRIMARY KEY(batch_id,doc_id)
) STRICT;
