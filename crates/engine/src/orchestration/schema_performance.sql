-- Keep hot outbox lookups proportional to runnable work, not retained history.
-- Partial indexes exclude the (deliberately durable) terminal rows entirely.
CREATE INDEX IF NOT EXISTS orchestration_effect_ready
    ON orchestration_effect_outbox(available_at,ordinal) WHERE status='pending';
CREATE INDEX IF NOT EXISTS orchestration_effect_expiry
    ON orchestration_effect_outbox(lease_expires_at) WHERE status='running';
CREATE INDEX IF NOT EXISTS orchestration_effect_lane_state
    ON orchestration_effect_outbox(thread_id,lane,status,ordinal);
CREATE INDEX IF NOT EXISTS orchestration_publication_pending
    ON orchestration_publication_batches(ordinal) WHERE status='pending';
