import { RefreshCw, Search } from "lucide-react";
import { useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router";
import { LoadError } from "@/components/experience/LoadError";
import { PersonCard } from "@/components/people/PersonCard";
import { Button, Segmented } from "@/components/ui/controls";
import { formatCount } from "@/lib/experience";
import { api } from "@/lib/ipc";
import { isActivePresence } from "@/lib/performance";
import { usePeople } from "@/lib/usePeople";
import { useNotifications } from "@/state/notifications";

type PeopleFilter = "all" | "active" | "playing";

const REFRESH_MS = 60_000;
const FILTERS: readonly { value: PeopleFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "active", label: "Online" },
  { value: "playing", label: "In game" },
];

export default function FriendsPage() {
  const { overview, error, reload, attempt } = usePeople(REFRESH_MS);
  const { notify } = useNotifications();
  const [username, setUsername] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<PeopleFilter>("all");

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (overview?.people ?? []).filter((person) => {
      const matches =
        !needle || person.displayName.toLowerCase().includes(needle) || person.username.toLowerCase().includes(needle);
      if (!matches) return false;
      if (filter === "active") return isActivePresence(person.presence);
      if (filter === "playing") return person.presence === "inGame";
      return true;
    });
  }, [overview, query, filter]);

  if (error && !overview) return <LoadError title="People didn't load" message={error} onRetry={() => void reload()} />;
  if (!overview) return <p className="px-8 py-8 text-muted">Loading people</p>;

  const profile = overview.profiles.find((item) => item.userId === overview.activeId);

  const add = async (event: FormEvent) => {
    event.preventDefault();
    setAdding(true);
    const failure = await attempt(() => api.addPerson(username));
    setAdding(false);
    setAddError(failure);
    if (!failure) setUsername("");
  };

  const remove = async (userId: number) => {
    const failure = await attempt(() => api.removePerson(userId));
    if (failure) notify({ tone: "error", title: "Person not removed", body: failure });
  };

  return (
    <div className="max-w-3xl px-8 py-8">
      <div className="flex items-center justify-between gap-4">
        <h1 className="font-display text-4xl font-bold tracking-tight">Friends</h1>
        {profile && (
          <Button onClick={() => void reload()}>
            <RefreshCw size={15} aria-hidden="true" />
            Refresh
          </Button>
        )}
      </div>
      <p className="mt-2 max-w-prose text-[13px] text-muted">
        Roblox only shares friends lists with a signed-in session, and BloxClient doesn't keep your Roblox login. Add the
        people you want to follow by username. Status comes from Roblox's public data, so private details can be missing.
      </p>

      {!profile && (
        <section className="mt-8">
          <h2 className="font-display text-xl font-semibold">Add your profile first</h2>
          <p className="mt-1 mb-4 max-w-prose text-muted">Your list is saved under your public Roblox username.</p>
          <Link
            to="/accounts"
            className="inline-flex h-9 items-center rounded-lg bg-accent px-3.5 text-[13px] font-medium text-on-accent hover:opacity-90"
          >
            Go to Accounts
          </Link>
        </section>
      )}

      {profile && (
        <>
          <p className="mt-4 text-[13px] text-muted">
            Showing {profile.displayName}'s list
            {profile.friendCount !== null && `. Roblox reports ${formatCount(profile.friendCount)} friends on this account`}
            .
          </p>

          <form onSubmit={(event) => void add(event)} className="mt-4 flex gap-2">
            <input
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              maxLength={21}
              aria-label="Roblox username to add"
              placeholder="Add by Roblox username"
              spellCheck={false}
              autoComplete="off"
              className="selectable h-9 min-w-0 flex-1 rounded-lg bg-raised px-3 text-[13px] text-text placeholder:text-faint"
            />
            <Button type="submit" variant="primary" disabled={adding || username.trim() === ""}>
              {adding ? "Adding" : "Add person"}
            </Button>
          </form>
          {addError && (
            <p role="alert" className="mt-2 text-[13px] text-danger">
              {addError}
            </p>
          )}

          {!overview.presenceAvailable && overview.people.length > 0 && (
            <p role="status" className="mt-3 text-[13px] text-danger">
              Roblox didn't return status right now. Names are from your saved list.
            </p>
          )}

          {overview.people.length > 0 && (
            <div className="mt-5 flex flex-wrap items-center gap-2">
              <div className="relative min-w-48 flex-1">
                <Search size={15} aria-hidden="true" className="absolute top-1/2 left-3 -translate-y-1/2 text-faint" />
                <input
                  type="search"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  aria-label="Search people"
                  placeholder="Search people"
                  spellCheck={false}
                  className="selectable h-9 w-full rounded-lg bg-raised pr-3 pl-9 text-[13px] text-text placeholder:text-faint"
                />
              </div>
              <Segmented label="Filter people" value={filter} options={FILTERS} onChange={setFilter} />
            </div>
          )}

          {overview.people.length === 0 && <p className="mt-6 text-muted">No one on your list yet.</p>}
          {overview.people.length > 0 && visible.length === 0 && (
            <p className="mt-6 text-muted">No one matches right now.</p>
          )}
          <ul className="mt-3">
            {visible.map((person) => (
              <PersonCard key={person.userId} person={person} onRemove={() => void remove(person.userId)} />
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
