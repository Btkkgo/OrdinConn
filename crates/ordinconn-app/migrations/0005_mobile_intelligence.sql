CREATE TABLE mobile_device_sessions (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  status TEXT NOT NULL,
  current_app TEXT,
  current_activity TEXT,
  connected_at TEXT NOT NULL,
  last_observation_at TEXT,
  domain_json TEXT NOT NULL
);

CREATE TABLE mobile_ui_snapshots (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES mobile_device_sessions(id) ON DELETE CASCADE,
  package_name TEXT NOT NULL,
  activity TEXT NOT NULL,
  ui_tree_hash TEXT NOT NULL,
  element_count INTEGER NOT NULL,
  captured_at TEXT NOT NULL,
  domain_json TEXT NOT NULL
);

CREATE TABLE mobile_observations (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  session_id TEXT NOT NULL REFERENCES mobile_device_sessions(id) ON DELETE RESTRICT,
  snapshot_id TEXT NOT NULL REFERENCES mobile_ui_snapshots(id) ON DELETE RESTRICT,
  package_name TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  frame_hash TEXT NOT NULL,
  ui_tree_hash TEXT NOT NULL,
  evidence_status TEXT NOT NULL CHECK (evidence_status IN ('observation_only','validated','rejected')),
  domain_json TEXT NOT NULL
);

CREATE TABLE warehouse_entries (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL UNIQUE,
  favorite INTEGER NOT NULL DEFAULT 0,
  saved INTEGER NOT NULL DEFAULT 0,
  tags_json TEXT NOT NULL DEFAULT '[]',
  note TEXT,
  collection_name TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

ALTER TABLE research_tasks ADD COLUMN mobile_packages_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE research_tasks ADD COLUMN mobile_budget_json TEXT NOT NULL DEFAULT '{}';

CREATE INDEX idx_mobile_observations_observed_at ON mobile_observations(observed_at DESC);
CREATE INDEX idx_mobile_snapshots_session ON mobile_ui_snapshots(session_id, captured_at DESC);
