import { NAV_ITEMS } from "@/app/navigation";
import { Select, SettingGroup, SettingRow, Switch } from "@/components/ui/controls";
import type { PageId } from "@/lib/bindings/PageId";
import { useSettings } from "@/state/settings";

const PAGE_OPTIONS = NAV_ITEMS.map((item) => ({ value: item.id, label: item.label }));

export function GeneralSection() {
  const { settings, update } = useSettings();
  const { general } = settings;

  return (
    <>
      <SettingGroup title="Startup">
        <SettingRow title="Launch on Windows startup" description="Open BloxClient when you sign in to Windows.">
          <Switch
            label="Launch on Windows startup"
            checked={general.launchOnStartup}
            onChange={(launchOnStartup) => void update({ general: { launchOnStartup } })}
          />
        </SettingRow>
        <SettingRow
          title="Start minimized"
          description="Open in the tray if minimize to tray is on, otherwise in the taskbar."
        >
          <Switch
            label="Start minimized"
            checked={general.startMinimized}
            onChange={(startMinimized) => void update({ general: { startMinimized } })}
          />
        </SettingRow>
        <SettingRow title="Default page" description="The page shown when BloxClient opens.">
          <Select<PageId>
            label="Default page"
            value={general.defaultPage}
            options={PAGE_OPTIONS}
            onChange={(defaultPage) => void update({ general: { defaultPage } })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Window">
        <SettingRow
          title="Minimize to tray"
          description="Closing the window keeps BloxClient in the system tray. When off, closing quits the app."
        >
          <Switch
            label="Minimize to tray"
            checked={general.minimizeToTray}
            onChange={(minimizeToTray) => void update({ general: { minimizeToTray } })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Language">
        <SettingRow title="App language" description="English is the only language available right now.">
          <span className="text-[13px] text-muted">English</span>
        </SettingRow>
      </SettingGroup>
    </>
  );
}
