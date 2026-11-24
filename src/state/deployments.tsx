import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { DeploymentOrigin } from "@/lib/bindings/DeploymentOrigin";
import type { InstalledDeployment } from "@/lib/bindings/InstalledDeployment";
import type { InstallProgress } from "@/lib/bindings/InstallProgress";
import { api, failureMessage, IpcFailure } from "@/lib/ipc";
import { useNotifications } from "./notifications";

interface DeploymentsValue {
  deployments: readonly InstalledDeployment[] | null;
  /** The version being installed. Survives navigating away from Settings. */
  installing: string | null;
  progress: InstallProgress | null;
  installError: { version: string; message: string } | null;
  refresh: () => Promise<void>;
  install: (version: string) => Promise<void>;
  cancel: () => void;
  remove: (version: string) => Promise<void>;
  launch: (version: string, origin: DeploymentOrigin) => Promise<void>;
}

const DeploymentsContext = createContext<DeploymentsValue | null>(null);

export function DeploymentsProvider({ children }: { children: ReactNode }) {
  const { notify } = useNotifications();
  const [deployments, setDeployments] = useState<InstalledDeployment[] | null>(null);
  const [installing, setInstalling] = useState<string | null>(null);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [installError, setInstallError] = useState<DeploymentsValue["installError"]>(null);

  useEffect(() => {
    const unlisten = listen<InstallProgress>("deployment-progress", (event) => setProgress(event.payload));
    return () => void unlisten.then((stop) => stop());
  }, []);

  const refresh = useCallback(async () => {
    try {
      setDeployments(await api.listDeployments());
    } catch (error) {
      notify({
        tone: "error",
        title: "Installed versions not loaded",
        body: failureMessage(error, "The list of installed versions couldn't be read."),
      });
    }
  }, [notify]);

  const install = useCallback(
    async (version: string) => {
      setInstalling(version);
      setProgress(null);
      setInstallError(null);
      try {
        await api.installDeployment(version);
        notify({ tone: "info", title: "Version installed", body: version });
        await refresh();
      } catch (error) {
        const message = failureMessage(error, "The install failed.");
        if (error instanceof IpcFailure && error.code === "deployment_cancelled") {
          notify({ tone: "info", title: "Download cancelled", body: version });
        } else {
          setInstallError({ version, message });
          notify({ tone: "error", title: "Version not installed", body: message });
        }
      } finally {
        setInstalling(null);
        setProgress(null);
      }
    },
    [notify, refresh],
  );

  const cancel = useCallback(() => void api.cancelDeploymentInstall(), []);

  const remove = useCallback(
    async (version: string) => {
      try {
        await api.removeDeployment(version);
        notify({ tone: "info", title: "Version removed", body: version });
      } catch (error) {
        notify({
          tone: "error",
          title: "Version not removed",
          body: failureMessage(error, "The version couldn't be removed."),
        });
      }
      await refresh();
    },
    [notify, refresh],
  );

  const launch = useCallback(
    async (version: string, origin: DeploymentOrigin) => {
      try {
        await api.launchDeployment(version, origin);
        notify({ tone: "info", title: "Roblox launched", body: version });
      } catch (error) {
        notify({
          tone: "error",
          title: "Roblox not launched",
          body: failureMessage(error, "This version couldn't be started."),
        });
      }
    },
    [notify],
  );

  const value = useMemo(
    () => ({ deployments, installing, progress, installError, refresh, install, cancel, remove, launch }),
    [deployments, installing, progress, installError, refresh, install, cancel, remove, launch],
  );
  return <DeploymentsContext.Provider value={value}>{children}</DeploymentsContext.Provider>;
}

export function useDeployments(): DeploymentsValue {
  const value = useContext(DeploymentsContext);
  if (!value) throw new Error("useDeployments must be used inside DeploymentsProvider");
  return value;
}
