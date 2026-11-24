import { Check, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Thumbnail } from "@/components/experience/Thumbnail";
import { Button } from "@/components/ui/controls";
import { ConfirmDialog } from "@/components/ui/FormDialogs";
import type { LocalProfileView } from "@/lib/bindings/LocalProfileView";
import { api } from "@/lib/ipc";
import { usePeople } from "@/lib/usePeople";
import { useNotifications } from "@/state/notifications";

export function LocalProfiles() {
  const { overview, error, attempt } = usePeople();
  const { notify } = useNotifications();
  const [username, setUsername] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [removing, setRemoving] = useState<LocalProfileView | null>(null);

  const add = async (event: FormEvent) => {
    event.preventDefault();
    setAdding(true);
    const failure = await attempt(() => api.addLocalProfile(username));
    setAdding(false);
    setFormError(failure);
    if (!failure) setUsername("");
  };

  const change = async (action: () => ReturnType<typeof api.people>, title: string) => {
    const failure = await attempt(action);
    if (failure) notify({ tone: "error", title, body: failure });
  };

  return (
    <section className="mt-10">
      <h2 className="font-display text-xl font-semibold">Profiles on this PC</h2>
      <p className="mt-1 max-w-prose text-[13px] text-muted">
        Add public usernames and switch between their Friends lists. This doesn't sign you in or change which account the
        Roblox Player uses.
      </p>

      {error && (
        <p role="alert" className="mt-2 text-[13px] text-danger">
          {error}
        </p>
      )}

      <ul className="mt-3">
        {overview?.profiles.map((profile) => {
          const active = profile.userId === overview.activeId;
          return (
            <li key={profile.userId} className="flex items-center gap-3 py-2">
              <Thumbnail url={profile.avatarUrl} name={profile.displayName} className="size-11 shrink-0 rounded-full" />
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{profile.displayName}</p>
                <p className="truncate text-[13px] text-muted">@{profile.username}</p>
              </div>
              {active ? (
                <span className="inline-flex shrink-0 items-center gap-1 text-[13px] text-muted">
                  <Check size={14} aria-hidden="true" />
                  Selected
                </span>
              ) : (
                <Button
                  aria-label={`Switch to ${profile.displayName}`}
                  onClick={() => void change(() => api.setActiveLocalProfile(profile.userId), "Profile not switched")}
                >
                  Switch
                </Button>
              )}
              <Button aria-label={`Remove ${profile.displayName}`} onClick={() => setRemoving(profile)}>
                <Trash2 size={15} aria-hidden="true" />
              </Button>
            </li>
          );
        })}
      </ul>

      <form onSubmit={(event) => void add(event)} className="mt-3 flex max-w-md gap-2">
        <input
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          maxLength={21}
          aria-label="Roblox username"
          placeholder="Roblox username"
          spellCheck={false}
          autoComplete="off"
          className="selectable h-9 min-w-0 flex-1 rounded-lg bg-raised px-3 text-[13px] text-text placeholder:text-faint"
        />
        <Button type="submit" disabled={adding || username.trim() === ""}>
          {adding ? "Adding" : "Add profile"}
        </Button>
      </form>
      {formError && (
        <p role="alert" className="mt-2 text-[13px] text-danger">
          {formError}
        </p>
      )}

      {removing && (
        <ConfirmDialog
          title={`Remove ${removing.displayName}?`}
          body="This also removes the people saved under this profile. Nothing changes on Roblox."
          action="Remove profile"
          onClose={() => setRemoving(null)}
          onConfirm={() => void change(() => api.removeLocalProfile(removing.userId), "Profile not removed")}
        />
      )}
    </section>
  );
}
