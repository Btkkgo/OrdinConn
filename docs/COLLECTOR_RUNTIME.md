# Collector Runtime

`collector-runtime` is transport-agnostic and implements REST, WebSocket, RSS/Atom, JSON Feed, and HTML article collection. Browser and Computer collectors are interface boundaries only in V0.1.

Every collector exposes `start`, `stop`, `fetch_once`, optional `subscribe`, and `health`. HTTP collection uses explicit timeouts, a bounded retry budget, exponential backoff, `Retry-After` for HTTP 429, a descriptive user agent, and a source-specific rate budget. Redirects are bounded and authentication redirects fail closed.

`CollectorScheduler` owns one single-flight task per enabled source plus a global concurrency semaphore. Cadence comes from `SourceDefinition`; start, pause, resume, persisted next-run state, bounded failure backoff, and awaited shutdown are explicit. Binance USDⓈ-M streaming subscribes to the BTC/ETH/SOL public mark-price, aggregate-trade, book-ticker, and finite-depth streams, replies to protocol pings, detects stale sessions, reconnects, and resubscribes.

Only public HTTP(S) and WS(S) endpoints pass policy validation. Login, paywall, CAPTCHA, cookie-gated, and private access are prohibited. `RawRecord` stores source identity, stable identifiers, canonical URL, timestamps, content type/hash, a content-addressed payload reference, and a bounded payload used for normalization.

Automated tests use local loopback HTTP and WebSocket fixtures. Run actual network checks explicitly with:

```sh
ORDINCONN_LIVE_SMOKE=1 cargo run -p ordinconn-app --example manual_live_smoke
ORDINCONN_SOAK_TEST=1 ORDINCONN_SOAK_SECONDS=1800 cargo run -p ordinconn-app --example manual_soak_test
```
