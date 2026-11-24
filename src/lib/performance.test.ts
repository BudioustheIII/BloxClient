import { describe, expect, it } from "vitest";
import { formatRate, isActivePresence, linePath, nearestIndex, presenceLabel, xPercent } from "./performance";

describe("formatRate", () => {
  it("scales units", () => {
    expect(formatRate(512)).toBe("512 B/s");
    expect(formatRate(2048)).toBe("2.0 KB/s");
    expect(formatRate(5 * 1024 * 1024)).toBe("5.0 MB/s");
    expect(formatRate(-5)).toBe("0 B/s");
  });
});

describe("linePath", () => {
  it("maps values onto the canvas", () => {
    const path = linePath(
      [
        { at: 0, value: 0 },
        { at: 10, value: 50 },
      ],
      100,
    );
    expect(path).toBe("M0.00 100.00L100.00 50.00");
  });

  it("breaks the line at missing values instead of inventing data", () => {
    const path = linePath(
      [
        { at: 0, value: 10 },
        { at: 5, value: null },
        { at: 10, value: 10 },
      ],
      10,
    );
    expect(path).toBe("M0.00 0.00M100.00 0.00");
  });

  it("caps values above the maximum", () => {
    expect(linePath([{ at: 0, value: 500 }], 100)).toBe("M100.00 0.00");
  });
});

describe("chart positions", () => {
  const points = [
    { at: 0, value: 1 },
    { at: 10, value: 2 },
    { at: 30, value: 3 },
  ];

  it("finds the nearest reading", () => {
    expect(nearestIndex(points, 0)).toBe(0);
    expect(nearestIndex(points, 0.4)).toBe(1);
    expect(nearestIndex(points, 2)).toBe(2);
    expect(nearestIndex([], 0.5)).toBe(-1);
  });

  it("handles a single point", () => {
    expect(xPercent(5, 5, 5)).toBe(100);
  });
});

describe("presence", () => {
  it("never claims someone is offline when Roblox can't say", () => {
    expect(presenceLabel("offline", null)).toBe("Offline or hidden");
    expect(presenceLabel("unknown", null)).toBe("Status unavailable");
  });

  it("shows a game only when Roblox reports one", () => {
    expect(presenceLabel("inGame", "Some Game")).toBe("Playing Some Game");
    expect(presenceLabel("inGame", null)).toBe("In a game");
  });

  it("counts playing and online as active", () => {
    expect(isActivePresence("inGame")).toBe(true);
    expect(isActivePresence("offline")).toBe(false);
  });
});
