CREATE TABLE scheduler_state (
    source_id TEXT PRIMARY KEY REFERENCES sources(id) ON DELETE RESTRICT,
    last_scheduled_at TEXT,
    last_started_at TEXT,
    last_completed_at TEXT,
    next_run_at TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    current_backoff_seconds INTEGER NOT NULL DEFAULT 0,
    last_result TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE market_metric_buckets (
    instrument_id TEXT NOT NULL,
    metric TEXT NOT NULL,
    window TEXT NOT NULL,
    bucket_time TEXT NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    sum REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    mean REAL NOT NULL,
    stddev REAL NOT NULL,
    volume_sum REAL NOT NULL,
    first_event_at TEXT NOT NULL,
    last_event_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(instrument_id, metric, window, bucket_time)
);
CREATE INDEX idx_metric_buckets_restore
    ON market_metric_buckets(instrument_id, metric, last_event_at);

CREATE TABLE evidence_cluster_members (
    cluster_id TEXT NOT NULL REFERENCES evidence_clusters(id) ON DELETE CASCADE,
    evidence_id TEXT NOT NULL REFERENCES evidence(id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL,
    canonical_url TEXT,
    original_url TEXT,
    content_hash TEXT,
    relation TEXT NOT NULL CHECK (relation IN ('original','syndication','independent','contradicting')),
    created_at TEXT NOT NULL,
    PRIMARY KEY(cluster_id, evidence_id)
);
CREATE INDEX idx_cluster_members_source ON evidence_cluster_members(cluster_id, source_id);

CREATE TABLE demand_timelines (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    topic TEXT NOT NULL CHECK (topic IN ('ai_infrastructure','gpu','memory','data_center')),
    direction TEXT NOT NULL CHECK (direction IN ('positive','negative','neutral')),
    time_bucket TEXT NOT NULL,
    evidence_id TEXT NOT NULL REFERENCES evidence(id) ON DELETE RESTRICT,
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(entity_id, topic, time_bucket, evidence_id)
);
CREATE INDEX idx_demand_timeline_lookup ON demand_timelines(entity_id, topic, observed_at);

CREATE TABLE intelligence_diagnostics (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    parse_failures INTEGER NOT NULL DEFAULT 0,
    reconnects INTEGER NOT NULL DEFAULT 0,
    stale_sources INTEGER NOT NULL DEFAULT 0,
    out_of_order_discards INTEGER NOT NULL DEFAULT 0,
    schema_drift INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

ALTER TABLE strategy_runs ADD COLUMN readiness TEXT NOT NULL DEFAULT 'ready';
ALTER TABLE strategy_runs ADD COLUMN instrument_id TEXT;
ALTER TABLE strategy_runs ADD COLUMN baseline_window TEXT;
ALTER TABLE strategy_runs ADD COLUMN input_snapshot_json TEXT NOT NULL DEFAULT 'null';
ALTER TABLE strategy_runs ADD COLUMN trigger_metrics_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE strategy_runs ADD COLUMN started_at TEXT;
ALTER TABLE strategy_runs ADD COLUMN completed_at TEXT;

ALTER TABLE signal_candidates ADD COLUMN data_origin TEXT NOT NULL DEFAULT 'UNKNOWN';
ALTER TABLE signals ADD COLUMN data_origin TEXT NOT NULL DEFAULT 'UNKNOWN';
ALTER TABLE signals ADD COLUMN published_at TEXT;
ALTER TABLE evidence ADD COLUMN data_origin TEXT NOT NULL DEFAULT 'UNKNOWN';
