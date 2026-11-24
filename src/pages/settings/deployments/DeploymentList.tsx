import { Play } from "lucide-react";
import { useEffect, useState } from "react";
import { Button, SettingGroup } from "@/components/ui/controls";
import type { InstalledDeployment } from "@/lib/bindings/InstalledDeployment";
import { formatBytes } from "@/lib/format";
import { useDeployments } from "@/state/deployments";

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

function details(item: InstalledDeployment): string {
  const parts = [item.status === "ready" ? "Ready" : "Incomplete"];
  if (item.installedAt !== null) parts.push(`installed ${dateFormat.format(new Date(item.installedAt * 1000))}`);
  if (item.sizeBytes !== null) parts.push(formatBytes(item.sizeBytes));
  return parts.join(", ");
}

function Row({ item }: { item: InstalledDeployment }) {
  const { launch, remove } = useDeployments();
  const [confirming, setConfirming] = useState(false);
  const [launching, setLaunching] = useState(false);

  const start = async () => {
    setLaunching(true);
    await launch(item.version, item.origin);
    setLaunching(false);
  };

  return (
    <li className="flex items-center justify-between gap-6 py-2.5">
      <div className="min-w-0">
        <p className="selectable font-medium">{item.version}</p>
        <p className={`text-[13px] ${item.status === "ready" ? "text-muted" : "text-danger"}`}>{details(item)}</p>
        <p className="selectable text-xs break-all text-faint">{item.path}</p>
      </div>
      <div className="flex shrink-0 gap-2">
        {confirming ? (
          <>
            <Button onClick={() => setConfirming(false)}>Keep</Button>
            <Button variant="danger" onClick={() => void remove(item.version)}>
              Remove
            </Button>
          </>
        ) : (
          <>
            <Button
              aria-label={`Launch ${item.version}`}
              disabled={item.status !== "ready" || launching}
              onClick={() => void start()}
            >
              <Play size={15} aria-hidden="true" />
              {launching ? "Launching" : "Launch"}
            </Button>
            {item.origin === "managed" && (
              <Button aria-label={`Remove ${item.version}`} onClick={() => setConfirming(true)}>
                Remove
              </Button>
            )}
          </>
        )}
      </div>
    </li>
  );
}

export function DeploymentList() {
  const { deployments, refresh } = useDeployments();

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (deployments === null) return null;
  const managed = deployments.filter((item) => item.origin === "managed");
  const roblox = deployments.filter((item) => item.origin === "roblox");

  return (
    <>
      <SettingGroup title="Installed by BloxClient">
        {managed.length === 0 ? (
          <p className="text-[13px] text-muted">Versions you download appear here.</p>
        ) : (
          <ul>
            {managed.map((item) => (
              <Row key={item.path} item={item} />
            ))}
          </ul>
        )}
      </SettingGroup>
      <SettingGroup title="Installed by Roblox">
        <p className="text-[13px] text-muted">
          {roblox.length === 0
            ? "No Roblox Player installs were found on this PC."
            : "BloxClient can launch these but never changes or removes them."}
        </p>
        <ul className="mt-1">
          {roblox.map((item) => (
            <Row key={item.path} item={item} />
          ))}
        </ul>
      </SettingGroup>
    </>
  );
}
