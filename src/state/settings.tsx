import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import type { Settings } from "@/lib/bindings/Settings";
import { api, IpcFailure, type SettingsPatch } from "@/lib/ipc";
import { useApplyAppearance } from "./useApplyAppearance";
import { useNotifications } from "./notifications";

interface SettingsValue {
  settings: Settings;
  /** Saves a change. Failures are reported in the notification center. */
  update: (patch: SettingsPatch) => Promise<boolean>;
  reset: () => Promise<boolean>;
}

const SettingsContext = createContext<SettingsValue | null>(null);

export function SettingsProvider({ initial, children }: { initial: Settings; children: ReactNode }) {
  const [settings, setSettings] = useState(initial);
  const { notify } = useNotifications();
  useApplyAppearance(settings.appearance);

  const run = useCallback(
    async (action: () => Promise<Settings>, failureTitle: string) => {
      try {
        setSettings(await action());
        return true;
      } catch (error) {
        const body = error instanceof IpcFailure ? error.message : undefined;
        notify({ tone: "error", title: failureTitle, body });
        return false;
      }
    },
    [notify],
  );

  const update = useCallback(
    (patch: SettingsPatch) => run(() => api.updateSettings(patch), "Setting not saved"),
    [run],
  );
  const reset = useCallback(() => run(api.resetSettings, "Settings not reset"), [run]);

  const value = useMemo(() => ({ settings, update, reset }), [settings, update, reset]);
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsValue {
  const value = useContext(SettingsContext);
  if (!value) throw new Error("useSettings must be used inside SettingsProvider");
  return value;
}
