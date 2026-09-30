import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MobileWorkspaceDto } from "@ordinconn/contracts";
import { createTranslator } from "../i18n";
import { WarehousePage } from "./WarehousePage";

describe("Repository presentation", () => {
  it("localizes filters and shows an actionable empty state without claiming saved items", () => {
    const workspace = { feed: [], warehouse: [] } as unknown as MobileWorkspaceDto;
    const html = renderToStaticMarkup(<WarehousePage workspace={workspace} onOpenDetail={() => undefined} onGoHome={() => undefined} t={createTranslator("zh-CN")} />);
    for (const copy of ["资料库", "全部", "收藏", "已保存", "暂无保存的数据", "返回首页"]) expect(html).toContain(copy);
    expect(html).not.toContain("清除筛选");
    expect(html).not.toContain("warehouse-card");
  });
});
