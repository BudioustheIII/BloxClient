import { useCallback, useEffect, useRef, useState } from "react";
import type { PeopleOverview } from "./bindings/PeopleOverview";
import { api, failureMessage } from "./ipc";

/**
 * Loads saved profiles and people. With `refreshMs`, presence updates only
 * while the window is visible.
 */
export function usePeople(refreshMs?: number) {
  const [overview, setOverview] = useState<PeopleOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const latest = useRef(0);

  const reload = useCallback(async () => {
    const request = ++latest.current;
    try {
      const next = await api.people();
      if (request === latest.current) {
        setOverview(next);
        setError(null);
      }
    } catch (reason) {
      if (request === latest.current) setError(failureMessage(reason, "Your people couldn't be loaded."));
    }
  }, []);

  useEffect(() => {
    void reload();
    if (!refreshMs) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void reload();
    }, refreshMs);
    return () => window.clearInterval(timer);
  }, [reload, refreshMs]);

  /** Runs a change and returns its error message. */
  const attempt = useCallback(async (action: () => Promise<PeopleOverview>): Promise<string | null> => {
    const request = ++latest.current;
    try {
      const next = await action();
      if (request === latest.current) setOverview(next);
      return null;
    } catch (reason) {
      return failureMessage(reason, "That change wasn't saved.");
    }
  }, []);

  return { overview, error, reload, attempt };
}
