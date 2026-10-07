-- Host-only input receipts. Mailbox enqueue, a local transcript boundary and
-- root-turn acceptance are not proof that an individual steer was accepted.
-- Keep these across projection rebuilds and process recovery.
-- Only newly admitted effects use this contract. Do not infer rejection or
-- replay older effects whose local success/acceptance semantics differed.
CREATE TABLE IF NOT EXISTS orchestration_steering_inputs (
    effect_id TEXT PRIMARY KEY REFERENCES orchestration_effect_outbox(effect_id),
    runtime_id TEXT
) STRICT;
-- Private process-incarnation binding. A replacement can reuse a logical
-- run/attempt/native conversation, but must not receive its old effects.
CREATE TABLE IF NOT EXISTS orchestration_runtime_targets (
    thread_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    run_attempt_id TEXT NOT NULL,
    root_node_id TEXT NOT NULL,
    provider_thread_id TEXT NOT NULL,
    runtime_id TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_steering_acceptances (
    effect_id TEXT PRIMARY KEY REFERENCES orchestration_effect_outbox(effect_id),
    thread_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    run_attempt_id TEXT NOT NULL,
    provider_session_id TEXT NOT NULL,
    accepted_at INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS orchestration_steering_acceptances_message
    ON orchestration_steering_acceptances(thread_id,message_id);
