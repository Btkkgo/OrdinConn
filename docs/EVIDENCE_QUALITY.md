# Evidence Quality

Evidence quality is computed by domain code, never by a model:

`0.35 reliability + 0.25 freshness + 0.20 completeness + 0.20 factual level + independent confirmation bonus - contradiction penalty`

The independent bonus is `0.05` per distinct independent source, capped at three. The contradiction penalty is `0.12` per contradiction, capped at three. Scores clamp to `0..1`.

Reliability tiers have deterministic defaults: Tier 1 official `0.95`, Tier 2 primary `0.85`, Tier 3 secondary `0.65`, Tier 4 unverified `0.35`. Freshness declines linearly between each policy's fresh and stale boundaries.

Clusters persist original, syndication, independent, and contradicting records. Syndication never counts as an independent confirmation; confirmation counts distinct canonical sources only. Summaries expose source count, independent confirmations, highest reliability, latest capture time, and contradictions. `MODEL_INFERENCE` cannot qualify as the sole publication Evidence.

Every Evidence item and Candidate declares `real`, `mock`, or `unknown` origin. A `real` Candidate fails closed if any linked Evidence is not `real`, preventing demo fixtures from entering a live Signal.
