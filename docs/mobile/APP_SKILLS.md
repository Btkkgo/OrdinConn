# Mobile App Skills

`MobileAppSkill` is a versioned, declarative contract containing a package matcher, known screens, recognizers, navigation rules, extractors, allowed and blocked actions, sensitive states, verification rules, and stop conditions.

M1 provides only `GenericAndroidSkill`, which recognizes a sanitized UI snapshot and extracts visible facts without navigation. Later phases add one application at a time in this order:

1. A single public-information skill after the runtime is stable.
2. `TelegramPublicSkill` for explicitly allowed public channels.
3. `XPublicSkill` for public posts and searches.
4. `BinancePublicSkill` for public market and announcement surfaces only.

Skills cannot bypass the application allowlist, Source Registry, Evidence gate, action policy, or research budgets.
