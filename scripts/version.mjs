// Keeps the app version the same everywhere it is recorded.
//   node scripts/version.mjs check
//   node scripts/version.mjs set 0.2.0
// An optional --root <folder> runs against a copy of the project.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SEMVER = /^\d+\.\d+\.\d+$/;

const args = process.argv.slice(2);
const rootFlag = args.indexOf("--root");
const root =
  rootFlag >= 0 ? args.splice(rootFlag, 2)[1] : join(dirname(fileURLToPath(import.meta.url)), "..");
const [command, nextVersion] = args;

/** Each pattern captures the text before the version, the version, and the text after it. */
const sources = [
  { file: "package.json", pattern: /^(\{[\s\S]*?\n  "version": ")([^"]+)(")/ },
  {
    file: "package-lock.json",
    optional: true,
    pattern: /^(\{[\s\S]*?\n  "version": ")([^"]+)(")/,
  },
  {
    file: "package-lock.json",
    optional: true,
    pattern: /("packages": \{\s*"": \{[\s\S]*?"version": ")([^"]+)(")/,
  },
  { file: "src-tauri/tauri.conf.json", pattern: /(\n  "version": ")([^"]+)(")/ },
  { file: "src-tauri/Cargo.toml", pattern: /(\[package\]\r?\nname = "bloxclient"\r?\nversion = ")([^"]+)(")/ },
  { file: "src-tauri/Cargo.lock", pattern: /(\nname = "bloxclient"\r?\nversion = ")([^"]+)(")/ },
];

function readVersions() {
  const found = [];
  for (const source of sources) {
    const path = join(root, source.file);
    if (!existsSync(path)) {
      if (source.optional) continue;
      throw new Error(`${source.file} is missing`);
    }
    const match = readFileSync(path, "utf8").match(source.pattern);
    if (!match) throw new Error(`Could not find the version in ${source.file}`);
    found.push({ file: source.file, version: match[2] });
  }
  return found;
}

function check() {
  const found = readVersions();
  const versions = new Set(found.map(({ version }) => version));
  if (versions.size !== 1) {
    for (const { file, version } of found) console.error(`${file}: ${version}`);
    throw new Error("Versions do not match");
  }
  const [version] = versions;
  if (!SEMVER.test(version)) throw new Error(`${version} is not a release version`);
  const changelog = join(root, "CHANGELOG.md");
  const notes = existsSync(changelog) ? readFileSync(changelog, "utf8") : "";
  if (!new RegExp(`^## ${version.replaceAll(".", "\\.")}( |$)`, "m").test(notes)) {
    throw new Error(`CHANGELOG.md has no section for ${version}`);
  }
  console.log(`Version ${version} is consistent and has release notes.`);
}

function set(version) {
  if (!SEMVER.test(version ?? "")) throw new Error("Usage: node scripts/version.mjs set 1.2.3");
  readVersions();
  for (const source of sources) {
    const path = join(root, source.file);
    if (source.optional && !existsSync(path)) continue;
    const text = readFileSync(path, "utf8");
    writeFileSync(path, text.replace(source.pattern, `$1${version}$3`));
  }
  console.log(`Set version to ${version}. Add its section to CHANGELOG.md before releasing.`);
}

try {
  if (command === "check") check();
  else if (command === "set") set(nextVersion);
  else throw new Error("Usage: node scripts/version.mjs <check|set> [version]");
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
