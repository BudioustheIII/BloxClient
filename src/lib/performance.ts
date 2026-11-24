import type { PresenceState } from "./bindings/PresenceState";

export interface ChartPoint {
  at: number;
  value: number | null;
}

const RATE_UNITS = ["B/s", "KB/s", "MB/s", "GB/s"] as const;

export function formatRate(bytesPerSecond: number): string {
  let value = Math.max(0, bytesPerSecond);
  let unit = 0;
  while (value >= 1024 && unit < RATE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${unit === 0 ? Math.round(value) : value.toFixed(1)} ${RATE_UNITS[unit]}`;
}

/** Horizontal position of a point, from 0 to 100. */
export function xPercent(at: number, first: number, last: number): number {
  return last > first ? ((at - first) / (last - first)) * 100 : 100;
}

/** An SVG path on a 100 by 100 canvas. Missing values break the line. */
export function linePath(points: readonly ChartPoint[], max: number): string {
  const first = points[0]?.at ?? 0;
  const last = points[points.length - 1]?.at ?? 0;
  const scale = max > 0 ? max : 1;
  let path = "";
  let drawing = false;
  for (const point of points) {
    if (point.value === null) {
      drawing = false;
      continue;
    }
    const x = xPercent(point.at, first, last).toFixed(2);
    const y = (100 - (Math.min(point.value, scale) / scale) * 100).toFixed(2);
    path += `${drawing ? "L" : "M"}${x} ${y}`;
    drawing = true;
  }
  return path;
}

export function nearestIndex(points: readonly ChartPoint[], fraction: number): number {
  if (points.length === 0) return -1;
  const first = points[0]!.at;
  const last = points[points.length - 1]!.at;
  const target = first + (last - first) * Math.min(1, Math.max(0, fraction));
  let best = 0;
  points.forEach((point, index) => {
    if (Math.abs(point.at - target) < Math.abs(points[best]!.at - target)) best = index;
  });
  return best;
}

export function presenceLabel(state: PresenceState, location: string | null): string {
  switch (state) {
    case "online":
      return "Online";
    case "inGame":
      return location ? `Playing ${location}` : "In a game";
    case "inStudio":
      return location ? `In Studio: ${location}` : "In Studio";
    case "invisible":
      return "Hidden";
    case "offline":
      return "Offline or hidden";
    case "unknown":
      return "Status unavailable";
  }
}

export function isActivePresence(state: PresenceState): boolean {
  return state === "online" || state === "inGame" || state === "inStudio";
}
