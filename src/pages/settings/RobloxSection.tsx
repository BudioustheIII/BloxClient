import { Select, SettingGroup, SettingRow } from "@/components/ui/controls";
import type { DeploymentHost } from "@/lib/bindings/DeploymentHost";
import { useSettings } from "@/state/settings";
import { DeploymentList } from "./deployments/DeploymentList";
import { VersionLookup } from "./deployments/VersionLookup";
import { InstallStatus } from "./InstallStatus";

const HOSTS: readonly { value: DeploymentHost; label: string }[] = [
  { value: "auto", label: "Automatic" },
  { value: "primary", label: "setup.rbxcdn.com" },
  { value: "aws", label: "setup-aws.rbxcdn.com" },
  { value: "akamai", label: "setup-ak.rbxcdn.com" },
];

export function RobloxSection() {
  const { settings, update } = useSettings();

  return (
    <>
      <InstallStatus />

      <SettingGroup title="Install a Roblox version">
        <VersionLookup />
      </SettingGroup>

      <SettingGroup title="Download source">
        <SettingRow
          title="Roblox server"
          description="All options are Roblox's official servers. Automatic switches server if one fails."
        >
          <Select
            label="Roblox download server"
            value={settings.roblox.deploymentHost}
            options={HOSTS}
            onChange={(deploymentHost) => void update({ roblox: { deploymentHost } })}
          />
        </SettingRow>
      </SettingGroup>

      <DeploymentList />
    </>
  );
}
