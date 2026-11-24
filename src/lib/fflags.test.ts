import { describe, expect, it } from "vitest";
import type { FlagView } from "./bindings/FlagView";
import { filterFlags, guessFlagKind } from "./fflags";

function flag(name: string, value: string): FlagView {
  return { name, value, kind: "string", enabled: true, listed: false, warnings: [] };
}

describe("guessFlagKind", () => {
  it("reads Roblox's naming convention", () => {
    expect(guessFlagKind("DFFlagThing")).toBe("boolean");
    expect(guessFlagKind("FIntThing")).toBe("number");
    expect(guessFlagKind("DFLogThing")).toBe("number");
    expect(guessFlagKind("SFStringThing")).toBe("string");
  });

  it("treats filter variants as text and unknown names as unknown", () => {
    expect(guessFlagKind("FFlagThing_PlaceFilter")).toBe("string");
    expect(guessFlagKind("Something")).toBeNull();
  });
});

describe("filterFlags", () => {
  const flags = [flag("FIntGrass", "10"), flag("FFlagSky", "true")];

  it("matches names and values without case sensitivity", () => {
    expect(filterFlags(flags, "grass").map((item) => item.name)).toEqual(["FIntGrass"]);
    expect(filterFlags(flags, "TRUE").map((item) => item.name)).toEqual(["FFlagSky"]);
  });

  it("returns everything for an empty search", () => {
    expect(filterFlags(flags, "  ")).toHaveLength(2);
  });
});
