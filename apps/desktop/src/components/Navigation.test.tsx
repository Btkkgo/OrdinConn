import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { createTranslator } from "../i18n";
import { Navigation } from "./Navigation";

describe("primary navigation", () => {
  it("exposes exactly Home, Warehouse, and Settings without an in-content logo", () => {
    const html = renderToStaticMarkup(
      <Navigation page="home" onNavigate={() => undefined} t={createTranslator("en")} />,
    );
    expect((html.match(/class="nav-item/g) ?? []).length).toBe(3);
    expect(html).toContain("Home");
    expect(html).toContain("Warehouse");
    expect(html).toContain("Settings");
    expect(html).not.toContain("OrdinConn");
    expect(html).not.toContain("V0.1");
    expect(html).not.toContain("<img");
    expect(html).not.toContain("brand-block");
    expect(html).toContain('aria-current="page"');
  });
});
