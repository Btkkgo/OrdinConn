# Signal Engine

Signal creation follows `Connector -> Evidence -> SignalCandidate -> Validation -> Published Signal`.

A Published Signal requires at least one non-inference Evidence item. Evidence relations identify primary, supporting, contradicting, and contextual records. Quality is deterministic and includes source reliability, freshness, factual level, confidence, relation role, and contradiction penalties.

Insufficient candidates remain `insufficient_evidence`. Contradictions stay visible and may reduce confidence or make a signal `watch`.

Readiness precedes this gate: a strategy that is warming, missing input, stale, schema-invalid, or history-insufficient creates no ordinary Candidate. A published live Signal records `data_origin=real`, strategy/version/parameters, baseline window, input and trigger metrics, observation/source/Evidence IDs, and publication time. Real publication rejects mock or unknown Evidence.
