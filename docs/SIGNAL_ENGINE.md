# Signal Engine

Signal creation follows `Connector -> Evidence -> SignalCandidate -> Validation -> Published Signal`.

A Published Signal requires at least one non-inference Evidence item. Evidence relations identify primary, supporting, contradicting, and contextual records. Quality is deterministic and includes source reliability, freshness, factual level, confidence, relation role, and contradiction penalties.

Insufficient candidates remain `insufficient_evidence`. Contradictions stay visible and may reduce confidence or make a signal `watch`.
