# Safety and Approval

V0.1 prohibits real-money execution, transfers, withdrawals, wallet signing, private-key access, seed-phrase access, and password-field capture.

A Trade Proposal is canonicalized with `CanonicalTradeProposalV1` and SHA-256. Every material risk field participates in the hash. Approval issues an opaque, cryptographically random, short-lived capability whose digest is stored server-side.

The capability is single-use, object-bound, version-bound, time-limited, and non-transferable. Validation, consumption, and starting Execution Record creation are atomic. Expiry, restart ambiguity, mutation, mismatch, reuse, and database failure all fail closed. Paper Execution follows the same path and never bypasses Approval.
