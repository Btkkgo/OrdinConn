# Data Retention

V0.1 retention is source-declared and enforced by migration-ready expiry fields:

- Raw payload excerpts: 7 days by default, bounded to 65,536 characters per record.
- Normalized observations: 365 days by default.
- In-memory metric history: hard bounded to 4,096 samples per instrument/metric series.
- Rolling history persistence: aggregate OHLC/statistical buckets only; streaming token/tick deltas are not written as audit events.
- Strategy definitions, parameter snapshots, Published Signals, Evidence references, and append-only audit events: retained until a future explicit maintenance policy exists.

Secrets, API keys, cookies, private content, full credentials, and unnecessary large payloads are never stored in `raw_records` or `runtime_events`. Cleanup is a future application maintenance service; no normal business path updates or deletes audit history. A later outbox migration can join the same state transaction without changing core protocols.
