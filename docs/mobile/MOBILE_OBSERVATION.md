# Mobile Observation

`MobileObservation` is the normalized result of one authorized mobile observation. It includes task and session identity, application/package/activity, semantic screen state, timestamps, frame and UI-tree hashes, source locator, visible facts, extracted entities, optional author and publication time, extraction method and confidence, redactions, privacy class, verification status, and metadata.

A screenshot is not a market fact. Agent summaries are not source evidence. `MobileObservation` therefore remains distinct from Evidence.

The promotion path is:

`MobileObservation -> deduplication -> source validation -> freshness -> reliability -> Evidence`

M1 persists observations with `observation_only` evidence state. It does not auto-promote them or create mobile-backed Signals.

Element references are generated per `MobileUISnapshot` as `@e1`, `@e2`, and so on. Any new snapshot invalidates every reference from the prior snapshot.
