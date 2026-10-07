-- Freeze control admission before another process can reuse a thread/attempt.
-- Missing legacy targets are never resolved against a newly started runtime.
CREATE TABLE IF NOT EXISTS orchestration_control_targets (
    effect_id TEXT PRIMARY KEY REFERENCES orchestration_effect_outbox(effect_id),
    target_json TEXT
) STRICT;
