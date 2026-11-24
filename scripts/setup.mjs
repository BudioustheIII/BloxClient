// Installs npm dependencies. From a OneDrive folder, packages are installed in
// %LOCALAPPDATA%\BloxClientBuild and linked back, because `npm install` run
// directly in the project replaces the link with a real, synced folder.
//
// Usage: npm run setup            install from the lockfile
//        npm run setup -- <pkg>   add a package
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, lstatSync, mkdirSync, rmSync, symlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npmCli = process.env.npm_execpath;
const packages = process.argv.slice(2);

if (!npmCli) {
  console.error("Run this through npm: npm run setup");
  process.exit(2);
}

function runNpm(cwd, args) {
  const result = spawnSync(process.execPath, [npmCli, ...args], { cwd, stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const installArgs = ["install", "--no-fund", "--no-audit", ...packages];

if (!process.env.LOCALAPPDATA || !/onedrive/i.test(root)) {
  runNpm(root, installArgs);
  process.exit(0);
}

const buildDir = join(process.env.LOCALAPPDATA, "BloxClientBuild");
const manifests = ["package.json", "package-lock.json"];
mkdirSync(buildDir, { recursive: true });

for (const file of manifests) {
  if (existsSync(join(root, file))) copyFileSync(join(root, file), join(buildDir, file));
}

runNpm(buildDir, installArgs);

for (const file of manifests) {
  copyFileSync(join(buildDir, file), join(root, file));
}

const link = join(root, "node_modules");
const stat = lstatSync(link, { throwIfNoEntry: false });
if (stat && !stat.isSymbolicLink()) rmSync(link, { recursive: true, force: true });
if (!stat || !stat.isSymbolicLink()) symlinkSync(join(buildDir, "node_modules"), link, "junction");

console.log(`Dependencies installed in ${buildDir}`);
