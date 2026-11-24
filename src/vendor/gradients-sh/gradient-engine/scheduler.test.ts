import { beforeEach, describe, expect, it } from "vitest";
import { noteFrame, noteGpuBusy, paceStats, recordPaint, resetPace } from "./scheduler";

const FRAME_MS = 1000 / 60;
let now = 0;

function runFrames(count: number, intervalMs: number, gpuBusy = false) {
  for (let i = 0; i < count; i += 1) {
    now += intervalMs;
    noteFrame(now);
    recordPaint(1);
    if (gpuBusy) noteGpuBusy();
  }
}

describe("frame pacing", () => {
  beforeEach(() => {
    now = 1000;
    resetPace();
    runFrames(40, FRAME_MS);
  });

  it("does not lower quality for idle display throttling", () => {
    runFrames(60, 50);
    expect(paceStats().cap).toBe(Number.POSITIVE_INFINITY);
  });

  it("lowers quality when long frames come from a busy GPU", () => {
    runFrames(20, 50, true);
    expect(paceStats().cap).toBeLessThan(Number.POSITIVE_INFINITY);
  });

  it("lowers quality when paints are held back at a steady display rate", () => {
    runFrames(30, FRAME_MS, true);
    expect(paceStats().cap).toBeLessThan(Number.POSITIVE_INFINITY);
  });
});
