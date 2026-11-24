import { GradientBackground } from "./gradient-surface";
import type { GradientBackgroundProps } from "./gradient-surface";
import { mercuryParams } from "./mercury-params";

export { mercuryParams };

/** The gradients.sh "Mercury" preset: liquid silver on near-black. */
export function Mercury(props: GradientBackgroundProps) {
  return <GradientBackground defaults={mercuryParams} {...props} />;
}
