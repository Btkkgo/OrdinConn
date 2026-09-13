# Crypto ABC

## A — On-chain

BTC/ETH on-chain observations use public blockchain sources. V0.1's real path uses public BTC mempool count, virtual size, and fees and feeds durable `BTC-NETWORK` 1h/24h baselines. A first observation remains neutral and produces a not-ready Strategy Run with no Candidate. Address watches store only a user label and address hash; exchange ownership is never guessed.

## B — Event and narrative

Official project, exchange, and regulator records can produce Event Candidates. Social adapters remain unavailable unless a legitimate public source is configured. A `NarrativeObservation` is research context, never a buy signal.

## C — Exchange

Binance public Spot and USDⓈ-M perpetual data covers BTCUSDT, ETHUSDT, and SOLUSDT only, with distinct `*-SPOT` and `*-PERP` IDs. Strategies cover funding anomaly, open-interest expansion, all four price/OI patterns, volume expansion, finite-depth order-book imbalance, and spread anomaly. Each requires its named metric and rolling baseline; missing, warming, stale, or schema-invalid data produces a not-ready run and no Candidate.
