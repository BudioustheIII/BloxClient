import { ExternalLink, X } from "lucide-react";
import { Link } from "react-router";
import { Thumbnail } from "@/components/experience/Thumbnail";
import type { PersonView } from "@/lib/bindings/PersonView";
import { api } from "@/lib/ipc";
import { isActivePresence, presenceLabel } from "@/lib/performance";
import { useNotifications } from "@/state/notifications";

const ICON_BUTTON =
  "grid size-8 place-items-center rounded-md text-muted transition-colors duration-(--fade) hover:bg-raised hover:text-text";

export function PersonCard({ person, onRemove }: { person: PersonView; onRemove?: () => void }) {
  const { notify } = useNotifications();
  const status = presenceLabel(person.presence, person.location);
  const active = isActivePresence(person.presence);

  const openProfile = () => {
    api.openUserProfile(person.userId).catch(() => {
      notify({ tone: "error", title: "Profile not opened", body: "Windows couldn't open your browser." });
    });
  };

  return (
    <li className="flex items-center gap-3 py-2">
      <span className="relative shrink-0">
        <Thumbnail url={person.avatarUrl} name={person.displayName} className="size-11 rounded-full" />
        <span
          aria-hidden="true"
          className={`absolute right-0 bottom-0 size-3 rounded-full border-2 border-surface ${
            active ? "bg-accent" : "bg-faint"
          }`}
        />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium">{person.displayName}</p>
        <p className="truncate text-[13px] text-muted">
          @{person.username} ·{" "}
          {person.universeId !== null ? (
            <Link to={`/experience/${person.universeId}`} className="underline underline-offset-2 hover:text-text">
              {status}
            </Link>
          ) : (
            status
          )}
        </p>
      </div>
      <button
        type="button"
        aria-label={`Open ${person.displayName}'s Roblox profile`}
        title="Open Roblox profile"
        className={ICON_BUTTON}
        onClick={openProfile}
      >
        <ExternalLink size={15} aria-hidden="true" />
      </button>
      {onRemove && (
        <button
          type="button"
          aria-label={`Remove ${person.displayName}`}
          title="Remove from this list"
          className={ICON_BUTTON}
          onClick={onRemove}
        >
          <X size={15} aria-hidden="true" />
        </button>
      )}
    </li>
  );
}
