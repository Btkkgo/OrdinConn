# Crypto ABC

## A — On-chain

BTC/ETH on-chain observations use public blockchain sources. V0.1's real path uses public BTC mempool count, virtual size, and fees. Strategy evaluation requires rolling pressure and fee baselines, so a first observation is retained as Evidence but rejected for insufficient history. Address watches store only a user label and address hash; exchange ownership is never guessed.

## B — Event and narrative

Official project, exchange, and regulator records can produce Event Candidates. Social adapters remain unavailable unless a legitimate public source is configured. A `NarrativeObservation` is research context, never a buy signal.

## C — Exchange

Binance public data covers BTCUSDT, ETHUSDT, and SOLUSDT. Strategies cover funding anomaly, open-interest expansion, price/OI divergence, volume expansion, order-book imbalance, and spread anomaly. Each requires its named metric and rolling baseline; missing data produces a rejected run, not a positive signal.

