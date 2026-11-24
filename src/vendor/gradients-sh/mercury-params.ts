import { kindDefaults } from "./gradient-engine";
import type { GradientParams } from "./gradient-engine";

/**
 * The gradients.sh "Mercury" preset: liquid silver on near-black. Kept apart
 * from the React component so the website can bundle it without React.
 */
export const mercuryParams: GradientParams = {
  ...kindDefaults("chrome"),
  background: "#040506",
  colors: ["#8c929c", "#ffffff", "#4a5060"],
  interaction: { pointer: 0.15, scroll: 0.2 },
  loopSeconds: 30,
  scale: 0.6,
  seed: 30,
  speed: 0.2,
  warp: 0.9,
};
