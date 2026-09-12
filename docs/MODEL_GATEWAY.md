# Model Gateway

Agent Runtime sends a provider-neutral request and receives normalized Model Events. Chat Completions wire fields are private to `OpenAICompatibleChatAdapter`.

Providers explicitly configure chat completions, Responses, streaming, tool calling, reasoning, vision, structured output, and JSON mode. Unsupported capabilities remain unavailable. V0.1 implements Chat Completions and retains interface boundaries for future adapters.

Credentials use OS secure storage and are referenced, never stored in SQLite or frontend storage.
