/** Minimal stand-in for the shadcn `cn` helper this component expects. */
export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(" ");
}
