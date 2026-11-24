import { Download, Play, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/controls";
import type { DeploymentInfo } from "@/lib/bindings/DeploymentInfo";
import type { InstallProgress } from "@/lib/bindings/InstallProgress";
import type { InstallStage } from "@/lib/bindings/InstallStage";
import { formatBytes } from "@/lib/format";
import { api, failureMessage } from "@/lib/ipc";
import { useDeployments } from "@/state/deployments";

const STAGE_LABELS: Record<InstallStage, string> = {
  preparing: "Preparing",
  downloading: "Downloading",
  extracting: "Unpacking",
  verifying: "Verifying files",
};

function ProgressBar({ progress, onCancel }: { progress: InstallProgress | null; onCancel: () => void }) {
  const percent =
    progress && progress.totalBytes > 0
      ? Math.min(100, Math.round((progress.doneBytes / progress.totalBytes) * 100))
      : 0;
  const stage = STAGE_LABELS[progress?.stage ?? "preparing"];

  return (
    <div className="mt-4">
      <div className="mb-1.5 flex items-baseline justify-between gap-4 text-[13px]">
        <span className="font-medium">{stage}</span>
        <span className="text-muted tabular-nums">
          {progress ? `${formatBytes(progress.doneBytes)} of ${formatBytes(progress.totalBytes)}` : "Starting"}
        </span>
      </div>
      <div
        role="progressbar"
        aria-label={stage}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className="h-1.5 overflow-hidden rounded-full bg-hover"
      >
        <div className="h-full rounded-full bg-accent" style={{ width: `${percent}%` }} />
      </div>
      <Button variant="inset" className="mt-3" onClick={onCancel}>
        <X size={15} aria-hidden="true" />
        Cancel download
      </Button>
    </div>
  );
}

export function VersionLookup() {
  const { deployments, installing, progress, installError, install, cancel, launch } = useDeployments();
  const [input, setInput] = useState("");
  const [checking, setChecking] = useState(false);
  const [info, setInfo] = useState<DeploymentInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setChecking(true);
    setInfo(null);
    setError(null);
    setConfirming(false);
    try {
      setInfo(await api.validateDeployment(input));
    } catch (reason) {
      setError(failureMessage(reason, "This version couldn't be checked."));
    } finally {
      setChecking(false);
    }
  };

  const installed = info ? (deployments ?? info.installed).filter((item) => item.version === info.version) : [];
  const ready = installed.find((item) => item.status === "ready");
  const isInstallingThis = info !== null && installing === info.version;
  const failure = info && installError?.version === info.version ? installError.message : null;

  return (
    <div>
      <p className="mb-3 max-w-prose text-[13px] text-muted">
        Enter a version hash to check it against Roblox's official servers. Roblox may not let older versions join games.
      </p>
      <form onSubmit={(event) => void submit(event)} className="flex gap-2">
        <input
          value={input}
          onChange={(event) => setInput(event.target.value)}
          aria-label="Version hash"
          placeholder="version-2c6fd5e09aaf4cfa"
          spellCheck={false}
          autoComplete="off"
          className="selectable h-9 min-w-0 flex-1 rounded-lg bg-raised px-3 text-[13px] text-text placeholder:text-faint"
        />
        <Button type="submit" disabled={checking || input.trim() === ""}>
          {checking ? "Checking" : "Validate"}
        </Button>
      </form>

      {error && (
        <p role="alert" className="mt-3 text-[13px] text-danger">
          {error}
        </p>
      )}

      {info && (
        <div className="fade-in mt-4 rounded-xl bg-raised p-4">
          <p className="selectable font-medium">{info.version}</p>
          <p className="text-[13px] text-muted">
            Available from Roblox's official server, {info.sourceHost}.{" "}
            {formatBytes(info.downloadBytes)} download, {formatBytes(info.installBytes)} installed,{" "}
            {info.fileCount.toLocaleString()} files.
          </p>

          {ready ? (
            <div className="mt-3">
              <p className="font-medium">Already installed</p>
              <p className="selectable mb-3 text-[13px] break-all text-muted">{ready.path}</p>
              <Button variant="inset" onClick={() => void launch(ready.version, ready.origin)}>
                <Play size={15} aria-hidden="true" />
                Launch
              </Button>
            </div>
          ) : isInstallingThis ? (
            <ProgressBar progress={progress} onCancel={cancel} />
          ) : confirming ? (
            <div className="mt-3">
              <p className="mb-2 text-[13px]">
                Download {formatBytes(info.downloadBytes)} and install this version in BloxClient's own folder? Your
                current Roblox install won't change.
              </p>
              <div className="flex gap-2">
                <Button variant="inset" onClick={() => setConfirming(false)}>
                  Not now
                </Button>
                <Button
                  variant="primary"
                  onClick={() => {
                    setConfirming(false);
                    void install(info.version);
                  }}
                >
                  <Download size={15} aria-hidden="true" />
                  Download
                </Button>
              </div>
            </div>
          ) : (
            <div className="mt-3">
              {failure && (
                <p role="alert" className="mb-2 text-[13px] text-danger">
                  {failure}
                </p>
              )}
              <Button variant="inset" disabled={installing !== null} onClick={() => setConfirming(true)}>
                <Download size={15} aria-hidden="true" />
                {installing ? "Another version is downloading" : "Download"}
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
