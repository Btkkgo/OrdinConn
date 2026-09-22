CREATE TABLE mobile_action_receipts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  snapshot_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('blocked','executed','failed')),
  verification TEXT,
  completed_at TEXT NOT NULL,
  domain_json TEXT NOT NULL
);

CREATE INDEX idx_mobile_action_receipts_completed_at ON mobile_action_receipts(completed_at DESC);
