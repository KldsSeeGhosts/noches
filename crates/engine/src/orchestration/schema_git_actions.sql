CREATE TABLE IF NOT EXISTS git_actions_kv (
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY(namespace,key)
) STRICT;
