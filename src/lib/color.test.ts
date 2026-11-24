import { describe, expect, it } from "vitest";
import { accentForTheme, luminance, readableInk } from "./color";

describe("accentForTheme", () => {
  it("keeps every accent on the dark theme", () => {
    expect(accentForTheme("#E8E8E6", "dark")).toBe("#E8E8E6");
  });

  it("replaces only near-white accents on the light theme", () => {
    expect(accentForTheme("#E8E8E6", "light")).toBe("#17181A");
    expect(accentForTheme("#FF7A66", "light")).toBe("#FF7A66");
  });
});

describe("luminance", () => {
  it("matches the WCAG endpoints", () => {
    expect(luminance("#000000")).toBe(0);
    expect(luminance("#FFFFFF")).toBeCloseTo(1, 5);
  });
});

describe("readableInk", () => {
  it("uses black on light accents", () => {
    expect(readableInk("#F4C542")).toBe("#000000");
    expect(readableInk("#E8E8E6")).toBe("#000000");
  });

  it("uses white on dark accents", () => {
    expect(readableInk("#1B1464")).toBe("#FFFFFF");
    expect(readableInk("#000000")).toBe("#FFFFFF");
  });
});
