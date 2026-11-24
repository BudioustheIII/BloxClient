import type { FlagKind } from "./bindings/FlagKind";
import type { FlagView } from "./bindings/FlagView";

/** Suggests a value type from Roblox's flag naming convention. */
export function guessFlagKind(name: string): FlagKind | null {
  const trimmed = name.trim();
  if (trimmed.endsWith("_PlaceFilter") || trimmed.endsWith("_DataCenterFilter")) return "string";
  if (/^(D|S)?FFlag/.test(trimmed)) return "boolean";
  if (/^(D|S)?FInt/.test(trimmed) || /^D?FLog/.test(trimmed)) return "number";
  if (/^(D|S)?FString/.test(trimmed)) return "string";
  return null;
}

export function filterFlags(flags: readonly FlagView[], query: string): FlagView[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...flags];
  return flags.filter((flag) => flag.name.toLowerCase().includes(needle) || flag.value.toLowerCase().includes(needle));
}
