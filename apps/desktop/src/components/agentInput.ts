export interface AgentInputKeyState {
  key: string;
  shiftKey: boolean;
  isComposing: boolean;
  busy: boolean;
  value: string;
}

export function shouldSendOnKeyDown(state: AgentInputKeyState): boolean {
  return (
    state.key === "Enter" &&
    !state.shiftKey &&
    !state.isComposing &&
    !state.busy &&
    state.value.trim().length > 0
  );
}
