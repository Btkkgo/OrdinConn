import { House, Library, Settings } from "lucide-react";
import type { Translator } from "../i18n";
import iconUrl from "../assets/ordinconn-icon-source.png";

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
      <div className="brand-block">
        <div className="brand-mark" aria-hidden="true"><img src={iconUrl} alt="" /></div>
      </div>
      <nav aria-label="Primary">
        {navigation.map(([id, label, Icon]) => (
          <button
            className={page === id ? "nav-item active" : "nav-item"}
            key={id}
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
