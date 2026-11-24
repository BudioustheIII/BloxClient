import { useCallback, useEffect, useState } from "react";
import type { FlagOverview } from "@/lib/bindings/FlagOverview";
import { api, failureMessage } from "@/lib/ipc";
import { useNotifications } from "@/state/notifications";

export type FlagAction = () => Promise<FlagOverview>;

export function useFlags() {
  const { notify } = useNotifications();
  const [overview, setOverview] = useState<FlagOverview | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const reload = useCallback(() => {
    setLoadError(null);
    api
      .fflags()
      .then(setOverview)
      .catch((reason: unknown) => setLoadError(failureMessage(reason, "Your flags couldn't be loaded.")));
  }, []);

  useEffect(reload, [reload]);

  /** Runs a change and returns its error message, for showing inside a dialog. */
  const attempt = useCallback(async (action: FlagAction): Promise<string | null> => {
    try {
      setOverview(await action());
      return null;
    } catch (reason) {
      return failureMessage(reason, "That change wasn't saved.");
    }
  }, []);

  /** Runs a change and reports any error as a notification. */
  const run = useCallback(
    async (action: FlagAction, failureTitle: string): Promise<boolean> => {
      const error = await attempt(action);
      if (error) notify({ tone: "error", title: failureTitle, body: error });
      return error === null;
    },
    [attempt, notify],
  );

  return { overview, loadError, reload, attempt, run };
}
