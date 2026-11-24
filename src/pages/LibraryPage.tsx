import { Pencil, Plus, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { Tile } from "@/components/experience/ExperienceTile";
import { Button } from "@/components/ui/controls";
import { ConfirmDialog, NameDialog } from "@/components/ui/FormDialogs";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import { formatRelativeTime, matchesGame, savedTarget } from "@/lib/experience";
import { useLibrary } from "@/state/library";
import { usePlay } from "@/state/play";

const ALL = "all";
const PINNED = "pinned";
const RECENT = "recent";

interface Entry {
  target: LaunchTarget;
  meta: string;
}

type Modal = "create" | "rename" | "delete" | null;

export default function LibraryPage() {
  const { library, createCollection, renameCollection, deleteCollection } = useLibrary();
  const { recent } = usePlay();
  const [tab, setTab] = useState(ALL);
  const [query, setQuery] = useState("");
  const [modal, setModal] = useState<Modal>(null);

  const games = library?.games ?? [];
  const collections = library?.collections ?? [];
  const collection = collections.find((item) => item.id === tab);
  const activeTab = collection || tab === PINNED || tab === RECENT ? tab : ALL;

  const entries = useMemo<Entry[]>(() => {
    if (activeTab === RECENT) {
      return recent.map((game) => ({
        target: savedTarget(game),
        meta: formatRelativeTime(game.lastPlayedAt),
      }));
    }
    const chosen =
      activeTab === ALL
        ? games
        : activeTab === PINNED
          ? games.filter((game) => game.pinned)
          : games.filter((game) => game.collectionIds.includes(activeTab));
    return [...chosen]
      .sort((a, b) => Number(b.pinned) - Number(a.pinned))
      .map((game) => ({
        target: savedTarget(game),
        meta: game.pinned ? "Pinned" : `Added ${formatRelativeTime(game.addedAt).toLowerCase()}`,
      }));
  }, [activeTab, games, recent]);

  const visible = entries.filter((entry) => matchesGame(entry.target, query));
  const tabs = [
    { id: ALL, label: "All saved" },
    { id: PINNED, label: "Pinned" },
    { id: RECENT, label: "Recently played" },
    ...collections.map((item) => ({ id: item.id, label: item.name })),
  ];

  return (
    <div className="px-8 py-8">
      <h1 className="font-display text-4xl font-bold tracking-tight">Library</h1>

      <div className="mt-5 flex flex-wrap items-center gap-2">
        <div className="relative w-full max-w-md">
          <Search size={16} aria-hidden="true" className="absolute top-1/2 left-3 -translate-y-1/2 text-faint" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            aria-label="Search your library"
            placeholder="Search your library"
            spellCheck={false}
            className="selectable h-10 w-full rounded-lg bg-raised pr-3 pl-9 text-text placeholder:text-faint"
          />
        </div>
        <Button onClick={() => setModal("create")}>
          <Plus size={15} aria-hidden="true" />
          New collection
        </Button>
        {collection && (
          <>
            <Button onClick={() => setModal("rename")}>
              <Pencil size={15} aria-hidden="true" />
              Rename
            </Button>
            <Button onClick={() => setModal("delete")}>
              <Trash2 size={15} aria-hidden="true" />
              Delete
            </Button>
          </>
        )}
      </div>

      <div role="tablist" aria-label="Library sections" className="mt-4 flex flex-wrap gap-1.5">
        {tabs.map((item) => {
          const selected = item.id === activeTab;
          return (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={selected}
              onClick={() => setTab(item.id)}
              className={`h-8 max-w-56 truncate rounded-full px-3.5 text-[13px] transition-colors duration-(--fade) ${
                selected ? "bg-accent font-semibold text-on-accent" : "bg-raised text-muted hover:text-text"
              }`}
            >
              {item.label}
            </button>
          );
        })}
      </div>

      {entries.length === 0 && (
        <p className="mt-8 max-w-prose text-muted">
          {activeTab === RECENT
            ? "Games you play will appear here."
            : "Nothing here yet. Open a game and use Favorite, Pin or Collections to save it."}
        </p>
      )}
      {entries.length > 0 && visible.length === 0 && <p className="mt-8 text-muted">No games match that search.</p>}

      <div className="mt-6 grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-x-4 gap-y-6">
        {visible.map(({ target, meta }) => (
          <Tile key={target.universeId} target={target} subtitle={target.creatorName} meta={<span>{meta}</span>} />
        ))}
      </div>

      {modal === "create" && (
        <NameDialog
          title="New collection"
          label="Collection name"
          initial=""
          action="Create collection"
          onClose={() => setModal(null)}
          onSubmit={createCollection}
        />
      )}
      {modal === "rename" && collection && (
        <NameDialog
          title="Rename collection"
          label="Collection name"
          initial={collection.name}
          action="Rename"
          onClose={() => setModal(null)}
          onSubmit={(name) => renameCollection(collection.id, name)}
        />
      )}
      {modal === "delete" && collection && (
        <ConfirmDialog
          title={`Delete ${collection.name}?`}
          body="Games stay in your favorites and pins. Games saved only in this collection are removed from your library."
          action="Delete collection"
          onClose={() => setModal(null)}
          onConfirm={() => {
            deleteCollection(collection.id);
            setTab(ALL);
          }}
        />
      )}
    </div>
  );
}
