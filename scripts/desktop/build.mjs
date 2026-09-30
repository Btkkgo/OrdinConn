import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
const args = process.argv.slice(2);
const command = process.platform === "darwin"
  ? ["python3", [fileURLToPath(new URL("macos_latest.py", import.meta.url)), ...args]]
  : [process.execPath, [`${root}node_modules/@tauri-apps/cli/tauri.js`, "build", ...args]];
const result = spawnSync(command[0], command[1], {
  cwd: `${root}apps/desktop`,
  stdio: "inherit",
});
if (result.error) console.error(`Desktop build failed: ${result.error.message}`);
process.exit(result.status ?? 1);
