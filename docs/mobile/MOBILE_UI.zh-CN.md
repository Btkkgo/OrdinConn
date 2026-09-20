# Mobile Intelligence UI

[English](MOBILE_UI.md) | [简体中文](MOBILE_UI.zh-CN.md)

## Navigation

左侧 Rail 只包含 Application Icon 与 Home、Warehouse、Settings 三个入口。Main Surface 以黑色为主，使用克制的紫色氛围，黄色只用于 Selection、Status 与 Primary Action。

## Home

Home 是联动的三栏 Workspace：

1. Intelligence Feed 列出 API、WebSocket、RSS、HTML、Browser、Desktop 与 Mobile Item，并显示 Source、Account/Channel、Summary、Time、Type、Evidence Status、Asset、Confidence 与 Favorite State。
2. Mobile Live View 显示 Device/Session State、Current App/Activity、Task、Runtime State、Latest Frame 与 Semantic Element Overlay。
3. Related Signals 优先显示与所选 Feed Item 相关的 Signal。不存在时显示 Evidence/Strategy Readiness，而不是虚构 Signal。

选择 Feed Item 后打开 Data Detail，显示 Source Content、Observation/Evidence Lineage、Discussion、Research、Related Signal、Favorite、Tag 与 Warehouse Action。

## Warehouse

Warehouse 是面向用户的 Research Library，不是 Database Console。它支持按 Source、Time、Market、Asset、Type、App、Evidence Quality、Favorite State、Signal Linkage、Contradiction State 与 Official-source State 搜索过滤。Saved Entry 保留对 Source Object、Associated Evidence、Signal、Task、Tag、Note 与 Collection 的引用。

## Settings

Settings 包含 Model、Strategy、Language、Text Size 与 Mobile Runtime。Text Scaling 支持 90%、100%、110%、120%，立即生效并持久化，但不缩放 Phone Frame。
