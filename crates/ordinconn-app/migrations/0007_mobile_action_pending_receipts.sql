CREATE TABLE mobile_action_receipts_next (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  snapshot_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending','blocked','executed','failed')),
  verification TEXT,
  completed_at TEXT NOT NULL,
  domain_json TEXT NOT NULL
);

INSERT INTO mobile_action_receipts_next (id,session_id,snapshot_id,status,verification,completed_at,domain_json)
SELECT id,session_id,snapshot_id,status,verification,completed_at,domain_json
FROM mobile_action_receipts;

DROP TABLE mobile_action_receipts;
ALTER TABLE mobile_action_receipts_next RENAME TO mobile_action_receipts;
CREATE INDEX idx_mobile_action_receipts_completed_at ON mobile_action_receipts(completed_at DESC);
