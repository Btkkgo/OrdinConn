# Agent Runtime

The runtime models Agent work as Thread, Turn, and Item. One Thread has at most one active Turn. Items represent completed user messages, Agent messages, tool calls, tool results, reports, and lifecycle events.

Runtime work is cancellable. Tauri commands create work and return identifiers without waiting for the model or tools. A normalized event bus streams progress. Token deltas are not persisted; completed items are.

Startup recovery marks unsafe unfinished work `interrupted` and records `runtime.task_interrupted`. Side-effecting tool calls are never replayed automatically.
