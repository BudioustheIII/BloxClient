// Entry for website/assets/mercury.js. Built by scripts/build-website.mjs, so the
// website's background is the app's own gradients.sh Mercury engine and preset.
import { mountGradient } from "../src/vendor/gradients-sh/gradient-engine";
import type { MountedGradient, MountOptions } from "../src/vendor/gradients-sh/gradient-engine";
import { mercuryParams } from "../src/vendor/gradients-sh/mercury-params";

export function mountMercury(canvas: HTMLCanvasElement, options?: MountOptions): MountedGradient {
  return mountGradient(canvas, mercuryParams, options);
}
