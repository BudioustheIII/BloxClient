import { Copy, Download, Pencil, Plus, RotateCcw, Trash2, Upload } from "lucide-react";
import { useState } from "react";
import { LoadError } from "@/components/experience/LoadError";
import { Button, Select, SettingGroup } from "@/components/ui/controls";
import { ConfirmDialog, NameDialog } from "@/components/ui/FormDialogs";
import type { FlagView } from "@/lib/bindings/FlagView";
import { formatRelativeTime } from "@/lib/experience";
import { api, failureMessage } from "@/lib/ipc";
import { useNotifications } from "@/state/notifications";
import { FlagDialog, ImportDialog } from "./FlagDialogs";
import { FlagList } from "./FlagList";
import { useFlags } from "./useFlags";

type Modal =
  | { kind: "add" }
  | { kind: "edit"; flag: FlagView }
  | { kind: "import" }
  | { kind: "new" }
  | { kind: "duplicate" }
  | { kind: "rename" }
  | { kind: "delete" }
  | { kind: "reset" }
  | { kind: "restore"; id: number; label: string };

export function FFlagsSection() {
  const { overview, loadError, reload, attempt, run } = useFlags();
  const { notify } = useNotifications();
  const [modal, setModal] = useState<Modal | null>(null);
  const close = () => setModal(null);

  if (loadError) return <LoadError title="Flags didn't load" message={loadError} onRetry={reload} />;
  if (!overview) return <p className="text-muted">Loading flags</p>;

  const profile = overview.profiles.find((item) => item.id === overview.activeId) ?? overview.profiles[0];
  if (!profile) return null;
  const readOnly = profile.source === "preset";
  const options = overview.profiles.map((item) => ({
    value: item.id,
    label: item.source === "preset" ? `${item.name} (recommended)` : item.name,
  }));

  const copyJson = async () => {
    try {
      await navigator.clipboard.writeText(await api.flagsJson(profile.id));
      notify({ tone: "info", title: "Flags copied", body: "Enabled flags are on your clipboard as JSON." });
    } catch (reason) {
      notify({ tone: "error", title: "Flags not copied", body: failureMessage(reason, "The clipboard wasn't available.") });
    }
  };

  const exportFile = async () => {
    try {
      const path = await api.exportFlagsFile(profile.id);
      notify({ tone: "info", title: "Flags exported", body: path });
    } catch (reason) {
      notify({ tone: "error", title: "Flags not exported", body: failureMessage(reason, "The file couldn't be saved.") });
    }
  };

  return (
    <>
      <SettingGroup title="FFlags">
        <p className="max-w-prose text-[13px] text-muted">
          BloxClient writes the selected profile only into versions it installed, right before you launch one from
          Settings &gt; Roblox. The Play button uses your registered launcher, which keeps its own flags. To use a profile
          there, export it and import it into that launcher.
        </p>
        <p className="mt-2 max-w-prose text-[13px] text-muted">
          Roblox decides which flags take effect and doesn't publish its current list. A flag marked Published list was
          allowed in September 2025, which doesn't guarantee it still works.
        </p>
      </SettingGroup>

      <SettingGroup title="Profile">
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <Select
            label="Active profile"
            value={profile.id}
            options={options}
            onChange={(id) => void run(() => api.setActiveFlagProfile(id), "Profile not changed")}
          />
          <Button onClick={() => setModal({ kind: "new" })}>
            <Plus size={15} aria-hidden="true" />
            New
          </Button>
          <Button onClick={() => setModal({ kind: "duplicate" })}>
            <Copy size={15} aria-hidden="true" />
            Duplicate
          </Button>
          {!readOnly && (
            <>
              <Button onClick={() => setModal({ kind: "rename" })}>
                <Pencil size={15} aria-hidden="true" />
                Rename
              </Button>
              <Button onClick={() => setModal({ kind: "delete" })}>
                <Trash2 size={15} aria-hidden="true" />
                Delete
              </Button>
            </>
          )}
        </div>
        {readOnly && (
          <p className="mt-2 text-[13px] text-muted">
            Recommended by BloxClient and read-only. Duplicate it to make your own version.
          </p>
        )}
      </SettingGroup>

      <SettingGroup title={readOnly ? "Recommended flags" : "Your flags"}>
        <FlagList
          flags={profile.flags}
          readOnly={readOnly}
          onAdd={() => setModal({ kind: "add" })}
          onToggle={(flag, enabled) =>
            void run(() => api.setFlagEnabled(profile.id, flag.name, enabled), "Flag not changed")
          }
          onEdit={(flag) => setModal({ kind: "edit", flag })}
          onRemove={(flag) => void run(() => api.removeFlag(profile.id, flag.name), "Flag not removed")}
        />
      </SettingGroup>

      <SettingGroup title="Import and export">
        <div className="mt-2 flex flex-wrap gap-2">
          <Button disabled={readOnly} onClick={() => setModal({ kind: "import" })}>
            <Upload size={15} aria-hidden="true" />
            Import
          </Button>
          <Button onClick={() => void exportFile()}>
            <Download size={15} aria-hidden="true" />
            Export to Downloads
          </Button>
          <Button onClick={() => void copyJson()}>
            <Copy size={15} aria-hidden="true" />
            Copy JSON
          </Button>
          <Button onClick={() => setModal({ kind: "reset" })}>
            <RotateCcw size={15} aria-hidden="true" />
            Reset to defaults
          </Button>
        </div>
        <p className="mt-2 text-xs text-faint">Exports include enabled flags only.</p>
      </SettingGroup>

      <SettingGroup title="Backups">
        {overview.backups.length === 0 ? (
          <p className="text-[13px] text-muted">Backups are saved automatically before changes.</p>
        ) : (
          <ul>
            {overview.backups.map((backup) => {
              const when = formatRelativeTime(backup.createdAt);
              return (
                <li key={backup.id} className="flex items-center justify-between gap-6 py-2">
                  <div className="min-w-0">
                    <p className="truncate font-medium">{backup.reason}</p>
                    <p className="text-[13px] text-muted">
                      {when}, {backup.profileCount} {backup.profileCount === 1 ? "profile" : "profiles"},{" "}
                      {backup.flagCount} {backup.flagCount === 1 ? "flag" : "flags"}
                    </p>
                  </div>
                  <Button
                    aria-label={`Restore backup from ${when}`}
                    onClick={() => setModal({ kind: "restore", id: backup.id, label: when })}
                  >
                    Restore
                  </Button>
                </li>
              );
            })}
          </ul>
        )}
      </SettingGroup>

      {(modal?.kind === "add" || modal?.kind === "edit") && (
        <FlagDialog
          flag={modal.kind === "edit" ? modal.flag : null}
          onClose={close}
          onSave={(input) =>
            attempt(() => api.saveFlag(profile.id, modal.kind === "edit" ? modal.flag.name : null, input))
          }
        />
      )}
      {modal?.kind === "import" && (
        <ImportDialog
          profileName={profile.name}
          onClose={close}
          onImport={(text, replace) => attempt(() => api.importFlags(profile.id, text, replace))}
        />
      )}
      {modal?.kind === "new" && (
        <NameDialog
          title="New profile"
          label="Profile name"
          initial=""
          action="Create profile"
          onClose={close}
          onSubmit={(name) => attempt(() => api.createFlagProfile(name, null))}
        />
      )}
      {modal?.kind === "duplicate" && (
        <NameDialog
          title={`Duplicate ${profile.name}`}
          label="Profile name"
          initial={`${profile.name} copy`.slice(0, 40)}
          action="Duplicate"
          onClose={close}
          onSubmit={(name) => attempt(() => api.createFlagProfile(name, profile.id))}
        />
      )}
      {modal?.kind === "rename" && (
        <NameDialog
          title="Rename profile"
          label="Profile name"
          initial={profile.name}
          action="Rename"
          onClose={close}
          onSubmit={(name) => attempt(() => api.renameFlagProfile(profile.id, name))}
        />
      )}
      {modal?.kind === "delete" && (
        <ConfirmDialog
          title={`Delete ${profile.name}?`}
          body="A backup is saved first, so you can restore this profile later."
          action="Delete profile"
          onClose={close}
          onConfirm={() => void run(() => api.deleteFlagProfile(profile.id), "Profile not deleted")}
        />
      )}
      {modal?.kind === "reset" && (
        <ConfirmDialog
          title="Reset all flags?"
          body="This removes your profiles and selects Default. A backup is saved first."
          action="Reset flags"
          onClose={close}
          onConfirm={() => void run(() => api.resetFlags(), "Flags not reset")}
        />
      )}
      {modal?.kind === "restore" && (
        <ConfirmDialog
          title="Restore this backup?"
          body={`Your profiles return to this saved state (${modal.label}). Your current setup is backed up first.`}
          action="Restore backup"
          onClose={close}
          onConfirm={() => void run(() => api.restoreFlagBackup(modal.id), "Backup not restored")}
        />
      )}
    </>
  );
}
