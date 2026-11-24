import { SettingGroup, SettingRow } from "@/components/ui/controls";
import { api } from "@/lib/ipc";
import { useResource } from "@/lib/useResource";

export function InstallStatus() {
  const { data: install, error } = useResource(() => api.robloxInstall(), "install");

  let value = "Checking";
  let description: string | undefined;
  let problem = false;

  if (error) {
    value = "Unknown";
    description = error;
    problem = true;
  } else if (install) {
    if (!install.launcherPath) {
      value = "Not found";
      description = "No program opens Roblox games on this PC. Install the Roblox Player to play.";
      problem = true;
    } else if (!install.launcherFound) {
      value = "Missing";
      description = `The registered program no longer exists: ${install.launcherPath}`;
      problem = true;
    } else {
      value = install.launcherName ?? "Roblox";
      description = install.isOfficialLauncher
        ? install.launcherPath
        : `Third-party launcher, set outside BloxClient. ${install.launcherPath}`;
    }
  }

  return (
    <SettingGroup title="Roblox on this PC">
      <SettingRow title="Games open with" description={description}>
        <span className={`shrink-0 text-[13px] font-medium ${problem ? "text-danger" : ""}`}>{value}</span>
      </SettingRow>
    </SettingGroup>
  );
}
