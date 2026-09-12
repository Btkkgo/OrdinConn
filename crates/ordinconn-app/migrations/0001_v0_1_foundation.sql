CREATE TABLE agent_threads (id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'active', created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE agent_turns (id TEXT PRIMARY KEY, thread_id TEXT NOT NULL REFERENCES agent_threads(id) ON DELETE CASCADE, status TEXT NOT NULL CHECK (status IN ('pending','running','waiting_tool','completed','failed','interrupted')), created_at TEXT NOT NULL, completed_at TEXT);
CREATE TABLE agent_items (id TEXT PRIMARY KEY, turn_id TEXT NOT NULL REFERENCES agent_turns(id) ON DELETE CASCADE, item_type TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('pending','completed','failed','interrupted')), content TEXT NOT NULL, model_id TEXT, created_at TEXT NOT NULL);

CREATE TABLE model_providers (id TEXT PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL, base_url TEXT NOT NULL, credential_ref TEXT, default_model TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 0, capabilities_json TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE connectors (id TEXT PRIMARY KEY, name TEXT NOT NULL, market TEXT NOT NULL, connector_type TEXT NOT NULL, capabilities_json TEXT NOT NULL, auth_type TEXT NOT NULL, status TEXT NOT NULL, reliability REAL NOT NULL, last_update TEXT NOT NULL, is_mock INTEGER NOT NULL DEFAULT 0);

CREATE TABLE evidence (id TEXT PRIMARY KEY, source TEXT NOT NULL, source_type TEXT NOT NULL, market TEXT NOT NULL, asset TEXT NOT NULL, title TEXT NOT NULL, content TEXT NOT NULL, raw_reference TEXT, captured_at TEXT NOT NULL, freshness REAL NOT NULL, reliability REAL NOT NULL, factual_level TEXT NOT NULL, confidence REAL NOT NULL, metadata_json TEXT NOT NULL, domain_json TEXT NOT NULL);
CREATE TABLE signal_candidates (id TEXT PRIMARY KEY, market TEXT NOT NULL, category TEXT NOT NULL, asset TEXT NOT NULL, title TEXT NOT NULL, summary TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE signals (id TEXT PRIMARY KEY, candidate_id TEXT NOT NULL, market TEXT NOT NULL, category TEXT NOT NULL, asset TEXT NOT NULL, title TEXT NOT NULL, summary TEXT NOT NULL, direction TEXT NOT NULL, confidence REAL NOT NULL, urgency REAL NOT NULL, time_horizon TEXT NOT NULL, evidence_quality REAL NOT NULL, agent_id TEXT NOT NULL, model_id TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, domain_json TEXT NOT NULL);
CREATE TABLE signal_evidence (signal_id TEXT NOT NULL REFERENCES signals(id) ON DELETE CASCADE, evidence_id TEXT NOT NULL REFERENCES evidence(id) ON DELETE RESTRICT, relation TEXT NOT NULL CHECK (relation IN ('primary','supporting','contradicting','context')), PRIMARY KEY (signal_id, evidence_id));

CREATE TABLE agent_reports (id TEXT PRIMARY KEY, signal_id TEXT NOT NULL REFERENCES signals(id) ON DELETE RESTRICT, executive_summary TEXT NOT NULL, evidence_summary TEXT NOT NULL, inference_summary TEXT NOT NULL, bull_case TEXT NOT NULL, bear_case TEXT NOT NULL, risk TEXT NOT NULL, invalidation TEXT NOT NULL, time_horizon TEXT NOT NULL, possible_actions TEXT NOT NULL, watch_conditions TEXT NOT NULL, conclusion TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE watchlists (id TEXT PRIMARY KEY, market TEXT NOT NULL, asset TEXT NOT NULL, label TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE trade_proposals (id TEXT PRIMARY KEY, signal_id TEXT NOT NULL REFERENCES signals(id) ON DELETE RESTRICT, version INTEGER NOT NULL, proposal_hash TEXT, proposal_hash_version INTEGER, status TEXT NOT NULL, domain_json TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE approval_requests (id TEXT PRIMARY KEY, proposal_id TEXT NOT NULL REFERENCES trade_proposals(id) ON DELETE RESTRICT, proposal_version INTEGER NOT NULL, proposal_hash TEXT NOT NULL, proposal_hash_version INTEGER NOT NULL, allowed_action TEXT NOT NULL, status TEXT NOT NULL, issued_at TEXT NOT NULL, expires_at TEXT NOT NULL, nonce TEXT NOT NULL, token_digest TEXT, token_state TEXT NOT NULL, issuer_session_id TEXT NOT NULL);
CREATE TABLE execution_records (id TEXT PRIMARY KEY, proposal_id TEXT NOT NULL REFERENCES trade_proposals(id) ON DELETE RESTRICT, approval_request_id TEXT NOT NULL UNIQUE REFERENCES approval_requests(id) ON DELETE RESTRICT, adapter_id TEXT NOT NULL, status TEXT NOT NULL, result_summary TEXT, created_at TEXT NOT NULL, completed_at TEXT);
CREATE TABLE automations (id TEXT PRIMARY KEY, name TEXT NOT NULL, schedule TEXT NOT NULL, target TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
CREATE TABLE settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL, updated_at TEXT NOT NULL);

CREATE TABLE runtime_events (id TEXT PRIMARY KEY, event_type TEXT NOT NULL, aggregate_type TEXT NOT NULL, aggregate_id TEXT NOT NULL, thread_id TEXT, turn_id TEXT, entity_id TEXT, payload_json TEXT NOT NULL, created_at TEXT NOT NULL, sequence INTEGER NOT NULL UNIQUE);

CREATE INDEX idx_signals_market_category ON signals(market, category);
CREATE INDEX idx_evidence_asset ON evidence(market, asset);
CREATE INDEX idx_runtime_events_aggregate ON runtime_events(aggregate_id, sequence);
CREATE INDEX idx_runtime_events_thread ON runtime_events(thread_id, sequence);

CREATE TRIGGER runtime_events_no_update BEFORE UPDATE ON runtime_events BEGIN SELECT RAISE(ABORT, 'runtime_events is append-only'); END;
CREATE TRIGGER runtime_events_no_delete BEFORE DELETE ON runtime_events BEGIN SELECT RAISE(ABORT, 'runtime_events is append-only'); END;

INSERT INTO settings (key, value_json, updated_at) VALUES ('language', '"en"', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
INSERT INTO automations (id, name, schedule, target, enabled, created_at) VALUES
('automation-btc-exchange', 'BTC Exchange Signal Scan', 'every 5 minutes', 'BTC exchange signals', 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
('automation-wallet-watch', 'Wallet Watchlist', 'hourly', 'crypto watchlist', 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
('automation-traditional-event', 'Traditional Finance Event Scan', 'before market open', 'traditional events', 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
