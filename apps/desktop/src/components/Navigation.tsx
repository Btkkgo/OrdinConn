import { House, Library, Settings } from "lucide-react";
import type { Translator } from "../i18n";

export type PageId = "home" | "warehouse" | "settings";

const navigation = [
  ["home", "nav.home", House],
  ["warehouse", "nav.warehouse", Library],
  ["settings", "nav.settings", Settings],
] as const;

interface NavigationProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
  t: Translator;
}

export function Navigation({ page, onNavigate, t }: NavigationProps) {
  return (
    <aside className="navigation">
      <nav aria-label={t("nav.primary")}>
        {navigation.map(([id, label, Icon]) => (
          <button
            className={page === id ? "nav-item active" : "nav-item"}
            key={id}
            title={t(label)}
            aria-current={page === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
            type="button"
          >
            <Icon aria-hidden="true" size={17} strokeWidth={1.8} />
            <span>{t(label)}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
