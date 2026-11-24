// Runs the Tauri CLI or cargo with a build environment that works from a
// OneDrive folder: Rust build output is kept outside the synced directory, and
// the finished NSIS installer is moved into src-tauri/target.
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const [tool, ...args] = process.argv.slice(2);

if (tool !== "tauri" && tool !== "cargo") {
  console.error("Usage: node scripts/run.mjs <tauri|cargo> [...args]");
  process.exit(2);
}

const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toLowerCase() === "path") ?? "PATH";
env[pathKey] = [join(homedir(), ".cargo", "bin"), env[pathKey]].join(delimiter);

if (!env.CARGO_TARGET_DIR && env.LOCALAPPDATA && /onedrive/i.test(root)) {
  env.CARGO_TARGET_DIR = join(env.LOCALAPPDATA, "BloxClientBuild", "target");
}

const [command, commandArgs] =
  tool === "tauri"
    ? [process.execPath, [join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"), ...args]]
    : ["cargo", args];

const result = spawnSync(command, commandArgs, { cwd: root, env, stdio: "inherit" });
if (result.error) {
  console.error(`Could not start ${tool}: ${result.error.message}`);
  process.exit(1);
}
if (result.status === 0 && tool === "tauri" && args[0] === "build" && env.CARGO_TARGET_DIR) {
  try {
    moveInstallers(env.CARGO_TARGET_DIR);
  } catch (error) {
    console.error(`Could not move the installer: ${error.message}`);
    process.exit(1);
  }
}
process.exit(result.status ?? 1);

function moveInstallers(targetDir) {
  const profile = args.includes("--debug") || args.includes("-d") ? "debug" : "release";
  const source = resolve(targetDir, profile, "bundle", "nsis");
  const destination = resolve(root, "src-tauri", "target", profile, "bundle", "nsis");
  if (source === destination || !existsSync(source)) return;

  mkdirSync(destination, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    const from = join(source, entry.name);
    const to = join(destination, entry.name);
    copyFileSync(from, to);
    rmSync(from);
    console.log(`Installer: ${to}`);
  }
}
