// Bundles the app's gradients.sh Mercury background into website/assets/mercury.js.
// It's a classic script, so the website also works when opened straight from disk.
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const assets = join(root, "website", "assets");

await build({
  configFile: false,
  root,
  publicDir: false,
  logLevel: "warn",
  build: {
    outDir: assets,
    emptyOutDir: false,
    copyPublicDir: false,
    target: "es2020",
    minify: true,
    lib: {
      entry: join(root, "scripts", "website-mercury.ts"),
      formats: ["iife"],
      name: "BloxMercury",
      fileName: () => "mercury.js",
    },
  },
});

// The minifier drops output banners, so the license notice is added afterwards.
const bundle = join(assets, "mercury.js");
const banner =
  "/*! gradients.sh gradient engine and Mercury preset, MIT licensed: https://gradients.sh/license. See NOTICE-gradients-sh.md. */\n";
writeFileSync(bundle, banner + readFileSync(bundle, "utf8"));

copyFileSync(join(root, "src", "vendor", "gradients-sh", "NOTICE.md"), join(assets, "NOTICE-gradients-sh.md"));
console.log("Built website/assets/mercury.js");
