function channel(value: number): number {
  const normalized = value / 255;
  return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance of a #RRGGBB color. */
export function luminance(hex: string): number {
  const red = channel(Number.parseInt(hex.slice(1, 3), 16));
  const green = channel(Number.parseInt(hex.slice(3, 5), 16));
  const blue = channel(Number.parseInt(hex.slice(5, 7), 16));
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

const LIGHT_THEME_INK = "#17181A";
const MAX_LIGHT_THEME_LUMINANCE = 0.7;

/** Near-white accents would vanish on the light theme, so they switch to ink. */
export function accentForTheme(accent: string, theme: "dark" | "light"): string {
  return theme === "light" && luminance(accent) > MAX_LIGHT_THEME_LUMINANCE ? LIGHT_THEME_INK : accent;
}

/** Picks black or white, whichever has the higher contrast on `hex`. */
export function readableInk(hex: string): "#000000" | "#FFFFFF" {
  const light = luminance(hex);
  const onBlack = (light + 0.05) / 0.05;
  const onWhite = 1.05 / (light + 0.05);
  return onBlack >= onWhite ? "#000000" : "#FFFFFF";
}
