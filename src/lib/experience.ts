import type { ExperienceCard } from "./bindings/ExperienceCard";
import type { LaunchTarget } from "./bindings/LaunchTarget";

export type ExperienceSort = "recommended" | "players" | "rating" | "name";

const COUNT_UNITS = ["", "K", "M", "B", "T"] as const;

export function formatCount(value: number): string {
  let scaled = Math.max(0, value);
  let unit = 0;
  while (scaled >= 1000 && unit < COUNT_UNITS.length - 1) {
    scaled /= 1000;
    unit += 1;
  }
  let rounded = unit === 0 ? Math.round(scaled) : Math.round(scaled * 10) / 10;
  if (rounded >= 1000 && unit < COUNT_UNITS.length - 1) {
    rounded = 1;
    unit += 1;
  }
  return `${rounded}${COUNT_UNITS[unit]}`;
}

/** Share of positive votes, or null when there are no votes to judge by. */
export function ratingPercent(up: number | null, down: number | null): number | null {
  if (up === null || down === null || up + down === 0) return null;
  return Math.round((up / (up + down)) * 100);
}

export function sortExperiences(items: readonly ExperienceCard[], sort: ExperienceSort): ExperienceCard[] {
  const sorted = [...items];
  switch (sort) {
    case "players":
      return sorted.sort((a, b) => (b.playing ?? -1) - (a.playing ?? -1));
    case "rating":
      return sorted.sort(
        (a, b) => (ratingPercent(b.upVotes, b.downVotes) ?? -1) - (ratingPercent(a.upVotes, a.downVotes) ?? -1),
      );
    case "name":
      return sorted.sort((a, b) => a.name.localeCompare(b.name));
    case "recommended":
      return sorted;
  }
}

export function formatRelativeTime(unixSeconds: number, nowMs: number = Date.now()): string {
  const seconds = Math.max(0, Math.floor(nowMs / 1000 - unixSeconds));
  if (seconds < 60) return "Just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hr ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return days === 1 ? "Yesterday" : `${days} days ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, { dateStyle: "medium" });
}

export function formatDuration(totalSeconds: number): string {
  const minutes = Math.floor(Math.max(0, totalSeconds) / 60);
  if (minutes < 1) return "Under a minute";
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${hours} hr` : `${hours} hr ${rest} min`;
}

/** Keeps only launch fields from a saved or recently played game. */
export function savedTarget(game: LaunchTarget): LaunchTarget {
  return {
    universeId: game.universeId,
    placeId: game.placeId,
    name: game.name,
    creatorName: game.creatorName,
    thumbnailUrl: game.thumbnailUrl,
  };
}

export function matchesGame(game: { name: string; creatorName: string | null }, query: string): boolean {
  const needle = query.trim().toLowerCase();
  return !needle || game.name.toLowerCase().includes(needle) || (game.creatorName ?? "").toLowerCase().includes(needle);
}

export function toLaunchTarget(card: ExperienceCard): LaunchTarget {
  return {
    universeId: card.universeId,
    placeId: card.placeId,
    name: card.name,
    creatorName: card.creatorName,
    thumbnailUrl: card.thumbnailUrl,
  };
}
