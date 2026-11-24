import { describe, expect, it } from "vitest";
import type { ExperienceCard } from "./bindings/ExperienceCard";
import { formatCount, formatDuration, formatRelativeTime, ratingPercent, sortExperiences } from "./experience";

function card(name: string, playing: number | null, up: number | null, down: number | null): ExperienceCard {
  return {
    universeId: name.length,
    placeId: 1,
    name,
    creatorName: null,
    creatorVerified: false,
    playing,
    upVotes: up,
    downVotes: down,
    thumbnailUrl: null,
    genre: null,
    maturity: null,
  };
}

describe("formatCount", () => {
  it("abbreviates each unit", () => {
    expect(formatCount(0)).toBe("0");
    expect(formatCount(999)).toBe("999");
    expect(formatCount(1500)).toBe("1.5K");
    expect(formatCount(4_399_000)).toBe("4.4M");
    expect(formatCount(8_097_805_488)).toBe("8.1B");
  });

  it("rolls over instead of showing 1000K", () => {
    expect(formatCount(999_960)).toBe("1M");
  });
});

describe("ratingPercent", () => {
  it("computes the positive share", () => {
    expect(ratingPercent(75, 25)).toBe(75);
  });

  it("returns null rather than inventing a rating", () => {
    expect(ratingPercent(0, 0)).toBeNull();
    expect(ratingPercent(null, 5)).toBeNull();
  });
});

describe("sortExperiences", () => {
  const items = [card("Beta", 10, 1, 9), card("Alpha", null, null, null), card("Gamma", 50, 9, 1)];

  it("keeps Roblox's order for recommended without mutating input", () => {
    expect(sortExperiences(items, "recommended").map((item) => item.name)).toEqual(["Beta", "Alpha", "Gamma"]);
    sortExperiences(items, "name");
    expect(items[0]?.name).toBe("Beta");
  });

  it("puts unknown values last", () => {
    expect(sortExperiences(items, "players").map((item) => item.name)).toEqual(["Gamma", "Beta", "Alpha"]);
    expect(sortExperiences(items, "rating").map((item) => item.name)).toEqual(["Gamma", "Beta", "Alpha"]);
  });

  it("sorts by name", () => {
    expect(sortExperiences(items, "name").map((item) => item.name)).toEqual(["Alpha", "Beta", "Gamma"]);
  });
});

describe("time formatting", () => {
  const now = 1_000_000_000_000;
  const ago = (seconds: number) => now / 1000 - seconds;

  it("describes recent times", () => {
    expect(formatRelativeTime(ago(10), now)).toBe("Just now");
    expect(formatRelativeTime(ago(300), now)).toBe("5 min ago");
    expect(formatRelativeTime(ago(7200), now)).toBe("2 hr ago");
    expect(formatRelativeTime(ago(86_400), now)).toBe("Yesterday");
    expect(formatRelativeTime(ago(259_200), now)).toBe("3 days ago");
  });

  it("formats durations", () => {
    expect(formatDuration(20)).toBe("Under a minute");
    expect(formatDuration(600)).toBe("10 min");
    expect(formatDuration(3600)).toBe("1 hr");
    expect(formatDuration(5400)).toBe("1 hr 30 min");
  });
});
