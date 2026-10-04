CREATE TABLE IF NOT EXISTS orchestration_launch_projects (
 id TEXT PRIMARY KEY, root TEXT NOT NULL, payload TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX IF NOT EXISTS launch_project_root ON orchestration_launch_projects(root)
 WHERE json_extract(payload,'$.deletedAt') IS NULL;
CREATE TABLE IF NOT EXISTS orchestration_launch_preferences (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), payload TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_launch_workflows (
 thread_id TEXT PRIMARY KEY, payload TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_launch_uploads (
 id TEXT PRIMARY KEY, payload TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS orchestration_launch_claims (
 id TEXT PRIMARY KEY, payload TEXT NOT NULL
) STRICT;
