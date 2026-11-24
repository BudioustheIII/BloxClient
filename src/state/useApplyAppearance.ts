import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect } from "react";
import type { AppearanceSettings } from "@/lib/bindings/AppearanceSettings";
import { accentForTheme, readableInk } from "@/lib/color";

const SYSTEM_DARK = "(prefers-color-scheme: dark)";

/** Mirrors appearance settings onto the document and webview. */
export function useApplyAppearance({ theme, accent, motion, uiScale }: AppearanceSettings) {
  useEffect(() => {
    const root = document.documentElement;
    const apply = (resolved: "dark" | "light") => {
      const visibleAccent = accentForTheme(accent, resolved);
      root.dataset.theme = resolved;
      root.style.setProperty("--accent", visibleAccent);
      root.style.setProperty("--on-accent", readableInk(visibleAccent));
    };

    if (theme !== "system") {
      apply(theme);
      return;
    }
    const query = window.matchMedia(SYSTEM_DARK);
    const onChange = () => apply(query.matches ? "dark" : "light");
    onChange();
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, [theme, accent]);

  useEffect(() => {
    document.documentElement.dataset.motion = motion;
  }, [motion]);

  useEffect(() => {
    // Native zoom keeps layout and hit-testing correct, unlike a CSS transform.
    getCurrentWebview()
      .setZoom(uiScale / 100)
      .catch((error: unknown) => console.error("Could not apply interface scale", error));
  }, [uiScale]);
}
