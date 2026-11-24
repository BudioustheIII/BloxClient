import { listen } from "@tauri-apps/api/event";
import { lazy, Suspense, useEffect, useState } from "react";
import { watchBackgroundAnimation } from "@/lib/backgroundAnimation";
import { api } from "@/lib/ipc";
import { useSettings } from "@/state/settings";

const Mercury = lazy(() => import("@/vendor/gradients-sh/mercury").then((module) => ({ default: module.Mercury })));

/**
 * The app-wide Mercury gradient. The canvas only exists while it can be seen
 * and Roblox is closed, so it never takes GPU time from a game. A still CSS
 * version underneath shows the rest of the time.
 */
export function AppBackdrop() {
  const { settings } = useSettings();
  const [allowed, setAllowed] = useState(false);

  useEffect(() => {
    return watchBackgroundAnimation(
      {
        read: api.backgroundAnimation,
        subscribe: (handler) => listen<boolean>("background-animation", ({ payload }) => handler(payload)),
      },
      setAllowed,
    );
  }, []);

  const animate = allowed && settings.appearance.motion === "full";

  return (
    <div aria-hidden="true" className="app-backdrop">
      {animate && (
        <Suspense fallback={null}>
          <Mercury className="size-full" maxDpr={1} maxFps={30} />
        </Suspense>
      )}
    </div>
  );
}
