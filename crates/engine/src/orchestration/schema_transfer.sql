-- Metadata is host-owned; refs are immutable and scoped to the checkout.
CREATE TABLE IF NOT EXISTS orchestration_file_checkpoints (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    run_id TEXT,
    phase TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    UNIQUE(thread_id, run_id, phase)
);
CREATE INDEX IF NOT EXISTS orchestration_file_checkpoint_timeline
    ON orchestration_file_checkpoints(thread_id, captured_at, id);
-- Separate from command receipts: a provider can accept before the host dies.
CREATE TABLE IF NOT EXISTS orchestration_transfer_delivery (
    transfer_id TEXT PRIMARY KEY,
    target_run_id TEXT NOT NULL,
    native_thread_id TEXT,
    status TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
