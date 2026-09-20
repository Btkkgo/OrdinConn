export type TextScale = 90 | 100 | 110 | 120;
export const textScaleStorageKey = "ordinconn.text-scale.v1";

export function normalizeTextScale(value: unknown): TextScale {
  return value === 90 || value === 100 || value === 110 || value === 120 ? value : 100;
}

export function applyTextScale(scale: TextScale): void {
  document.documentElement.style.setProperty("--ui-scale", String(scale / 100));
}
