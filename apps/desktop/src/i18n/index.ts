import { en } from "./en";
import { zhCN } from "./zh-CN";

export type Locale = "en" | "zh-CN";
export const availableLocales: Locale[] = ["en", "zh-CN"];
export const localeStorageKey = "ordinconn.locale.v1";

export function resolveLocale(value: string | null | undefined): Locale {
  return value === "zh-CN" ? "zh-CN" : "en";
}

const dictionaries: Record<Locale, Record<string, string>> = { en, "zh-CN": zhCN };

export type Translator = (key: string, replacements?: Record<string, string | number>) => string;

export function createTranslator(locale: Locale = "en"): Translator {
  return (key, replacements = {}) => {
    const template = dictionaries[locale][key] ?? en[key as keyof typeof en] ?? `[${key}]`;
    return Object.entries(replacements).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      template,
    );
  };
}
