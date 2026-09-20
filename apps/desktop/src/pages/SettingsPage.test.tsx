import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MobileWorkspaceDto } from "@ordinconn/contracts";
import { createTranslator } from "../i18n";
import { SettingsPage } from "./SettingsPage";

const workspace: MobileWorkspaceDto = {
  runtimeStatus: "disconnected",
  adbStatus: "ready",
  androidEnvironment: {
    sdkStatus: "detected",
    adbStatus: "ready",
    emulatorStatus: "ready",
    sdkRoot: "~/Library/Android/sdk",
    adbPath: "~/Library/Android/sdk/platform-tools/adb",
    emulatorPath: "~/Library/Android/sdk/emulator/emulator",
    sdkmanagerPath: "~/Library/Android/sdk/cmdline-tools/latest/bin/sdkmanager",
    avdmanagerPath: "~/Library/Android/sdk/cmdline-tools/latest/bin/avdmanager",
    adbVersion: "Android Debug Bridge version 1.0.41",
    availableAvds: [{ name: "Pixel_9_API_36", status: "stopped", deviceProfile: "pixel_9", architecture: "arm64-v8a", running: false }],
    onlineDevices: [],
  },
  observations: [], feed: [], warehouse: [], strategies: [],
  settings: { allowedApps: [], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } },
};

describe("mobile runtime settings diagnostics", () => {
  it("renders truthful tool readiness, counts, paths, and existing AVD controls", () => {
    const html = renderToStaticMarkup(
      <SettingsPage
        locale="en" onLocaleChange={() => undefined} textScale={100} onTextScaleChange={() => undefined}
        workspace={workspace} providers={[]} onSaveProvider={async () => undefined} onTestProvider={async () => ({ ok: true, message: "ok" })}
        onSaveMobile={async () => undefined} onStartAvd={async () => undefined} onStrategyChange={async () => undefined} t={createTranslator("en")}
      />,
    );

    expect(html).toContain("Android SDK");
    expect(html).toContain("Detected");
    expect(html).toContain("ADB");
    expect(html).toContain("Emulator");
    expect(html).toContain("AVDs");
    expect(html).toContain("Connected devices");
    expect(html).toContain("~/Library/Android/sdk/platform-tools/adb");
    expect(html).toContain("Pixel_9_API_36");
    expect(html).toContain("Start AVD");
    expect(html).not.toContain("Tap element");
  });
});
