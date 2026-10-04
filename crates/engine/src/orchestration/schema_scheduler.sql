-- Host-only definitions and dispatch claims; not a replica-side execution queue.
CREATE TABLE IF NOT EXISTS orchestration_scheduled_tasks (
    task_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    next_run_at TEXT,
    updated_at TEXT NOT NULL,
    last_run_status TEXT NOT NULL,
    last_run_at TEXT,
    last_run_error TEXT,
    run_count INTEGER NOT NULL DEFAULT 0,
    active_claim_id TEXT,
    task_json TEXT NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS orchestration_scheduled_due
    ON orchestration_scheduled_tasks(enabled, last_run_status, next_run_at, task_id);
CREATE INDEX IF NOT EXISTS orchestration_scheduled_project
    ON orchestration_scheduled_tasks(project_id, updated_at);
CREATE TABLE IF NOT EXISTS orchestration_scheduled_runs (
    claim_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    trigger TEXT NOT NULL CHECK(trigger IN ('scheduled', 'manual')),
    command_id TEXT NOT NULL UNIQUE,
    message_id TEXT NOT NULL UNIQUE,
    host_id TEXT NOT NULL,
    task_json TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL CHECK(status IN ('running', 'succeeded', 'failed')),
    error TEXT
) STRICT;
CREATE INDEX IF NOT EXISTS orchestration_scheduled_run_history
    ON orchestration_scheduled_runs(task_id, started_at, claim_id);
