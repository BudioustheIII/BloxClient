import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { Button, SettingGroup, SettingRow } from "@/components/ui/controls";
import { formatBytes } from "@/lib/format";
import { api, failureMessage } from "@/lib/ipc";
import { useLibrary } from "@/state/library";
import { useNotifications } from "@/state/notifications";
import { usePlay } from "@/state/play";
import { useDiagnostics } from "./useDiagnostics";

function ClearHistory() {
  const { recent, clearRecent } = usePlay();
  const [confirming, setConfirming] = useState(false);

  if (confirming) {
    return (
      <div className="flex shrink-0 gap-2">
        <Button onClick={() => setConfirming(false)}>Keep history</Button>
        <Button
          variant="danger"
          onClick={() => {
            setConfirming(false);
            void clearRecent();
          }}
        >
          Clear history
        </Button>
      </div>
    );
  }
  return (
    <Button disabled={recent.length === 0} onClick={() => setConfirming(true)}>
      Clear history
    </Button>
  );
}

function ClearData({ label, onClear }: { label: string; onClear: () => Promise<void> }) {
  const [confirming, setConfirming] = useState(false);
  if (!confirming) return <Button onClick={() => setConfirming(true)}>{label}</Button>;
  return (
    <div className="flex shrink-0 gap-2">
      <Button onClick={() => setConfirming(false)}>Keep</Button>
      <Button
        variant="danger"
        onClick={() => {
          setConfirming(false);
          void onClear();
        }}
      >
        {label}
      </Button>
    </div>
  );
}

export function PrivacySection() {
  const { diagnostics, error } = useDiagnostics();
  const { notify } = useNotifications();
  const { recent } = usePlay();
  const { clear: clearLibrary } = useLibrary();

  const clearPeople = async () => {
    try {
      await api.clearPeople();
      notify({ tone: "info", title: "People cleared" });
    } catch (reason) {
      notify({ tone: "error", title: "People not cleared", body: failureMessage(reason, "They couldn't be removed.") });
    }
  };

  const openData = () => {
    api.openFolder("data").catch((reason: unknown) => {
      notify({ tone: "error", title: "Folder not opened", body: failureMessage(reason, "Windows couldn't open it.") });
    });
  };

  return (
    <>
      <SettingGroup title="Network activity">
        <SettingRow
          title="Game information"
          description="Home and Discover load public game data from games.roblox.com, thumbnails.roblox.com, apis.roblox.com and tr.rbxcdn.com. No account details are sent."
        >
          <span className="shrink-0 text-[13px] text-muted">When browsing</span>
        </SettingRow>
        <SettingRow
          title="Roblox versions"
          description="Version checks and downloads use setup.rbxcdn.com, setup-aws.rbxcdn.com and setup-ak.rbxcdn.com."
        >
          <span className="shrink-0 text-[13px] text-muted">On request</span>
        </SettingRow>
        <SettingRow
          title="Roblox sign-in"
          description="Signing in uses your browser and apis.roblox.com. BloxClient stores no password, cookie or sign-in token."
        >
          <span className="shrink-0 text-[13px] text-muted">When you sign in</span>
        </SettingRow>
        <SettingRow
          title="Friends and profiles"
          description="Usernames you add are looked up on users.roblox.com, friends.roblox.com, presence.roblox.com and thumbnails.roblox.com. No login is sent."
        >
          <span className="shrink-0 text-[13px] text-muted">When viewing people</span>
        </SettingRow>
        <SettingRow
          title="Connection check"
          description="Run check on the Performance page contacts www.roblox.com, apis.roblox.com, setup.rbxcdn.com and tr.rbxcdn.com."
        >
          <span className="shrink-0 text-[13px] text-muted">On request</span>
        </SettingRow>
        <SettingRow title="Analytics" description="BloxClient doesn't include analytics or tracking.">
          <span className="shrink-0 text-[13px] text-muted">None</span>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Data on this PC">
        <SettingRow
          title="Play history"
          description={`${recent.length} recently played ${recent.length === 1 ? "game" : "games"}. Continue playing reads which games you joined from Roblox's logs on this PC. Only game IDs and times are used, and your history is encrypted on this PC.`}
        >
          <ClearHistory />
        </SettingRow>
        <SettingRow
          title="Saved games"
          description="Favorites, pins and collections, encrypted on this PC."
        >
          <ClearData label="Clear saved games" onClear={clearLibrary} />
        </SettingRow>
        <SettingRow title="Profiles and people" description="Usernames you added, encrypted on this PC.">
          <ClearData label="Clear people" onClear={clearPeople} />
        </SettingRow>
        <SettingRow
          title="Local database"
          description={error ?? (diagnostics ? formatBytes(diagnostics.databaseBytes) : "Checking size")}
        >
          <Button onClick={openData}>
            <FolderOpen size={15} aria-hidden="true" />
            Open data folder
          </Button>
        </SettingRow>
      </SettingGroup>
    </>
  );
}
