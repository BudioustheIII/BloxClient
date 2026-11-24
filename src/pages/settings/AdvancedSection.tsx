import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { Button, SettingGroup, SettingRow } from "@/components/ui/controls";
import { api, IpcFailure } from "@/lib/ipc";
import { formatBytes } from "@/lib/format";
import { useNotifications } from "@/state/notifications";
import { useSettings } from "@/state/settings";
import { useDiagnostics } from "./useDiagnostics";

export function AdvancedSection() {
  const { diagnostics, error } = useDiagnostics();
  const { reset } = useSettings();
  const { notify } = useNotifications();
  const [confirming, setConfirming] = useState(false);

  const openLogs = () => {
    api.openFolder("logs").catch((reason: unknown) => {
      notify({
        tone: "error",
        title: "Folder not opened",
        body: reason instanceof IpcFailure ? reason.message : undefined,
      });
    });
  };

  const confirmReset = async () => {
    setConfirming(false);
    if (await reset()) notify({ tone: "info", title: "Settings reset" });
  };

  const rows: readonly (readonly [string, string])[] = diagnostics
    ? [
        ["App version", diagnostics.version],
        ["Database schema", String(diagnostics.schemaVersion)],
        ["Database size", formatBytes(diagnostics.databaseBytes)],
        ["WebView2", diagnostics.webviewVersion ?? "Not detected"],
        ["Platform", diagnostics.platform],
        ["Data folder", diagnostics.dataDir],
        ["Log folder", diagnostics.logDir],
      ]
    : [];

  return (
    <>
      <SettingGroup title="Diagnostics">
        {error && <p className="text-danger">{error}</p>}
        <dl className="mt-2 grid grid-cols-[9rem_minmax(0,1fr)] gap-y-1.5 text-[13px]">
          {rows.map(([label, value]) => (
            <div key={label} className="contents">
              <dt className="text-muted">{label}</dt>
              <dd className="selectable break-all">{value}</dd>
            </div>
          ))}
        </dl>
      </SettingGroup>

      <SettingGroup title="Logs">
        <SettingRow title="Application logs" description="Logs stay on this PC and never contain sign-in details.">
          <Button onClick={openLogs}>
            <FolderOpen size={15} aria-hidden="true" />
            Open log folder
          </Button>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Reset">
        <SettingRow title="Reset settings" description="Restores every setting to its default.">
          {confirming ? (
            <div className="flex shrink-0 gap-2">
              <Button onClick={() => setConfirming(false)}>Keep settings</Button>
              <Button variant="danger" onClick={() => void confirmReset()}>
                Reset settings
              </Button>
            </div>
          ) : (
            <Button onClick={() => setConfirming(true)}>Reset settings</Button>
          )}
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Credits">
        <p className="max-w-prose text-[13px] text-muted">
          Emoji graphics by Twemoji, copyright Twitter, Inc. and contributors, licensed under CC BY 4.0. Animated
          background by gradients.sh, MIT licensed. Full notices are in the app's licenses folder.
        </p>
      </SettingGroup>
    </>
  );
}
