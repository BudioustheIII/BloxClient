import { FolderPlus, Heart, Pin } from "lucide-react";
import { useState } from "react";
import { Button, Switch } from "@/components/ui/controls";
import { Dialog } from "@/components/ui/Dialog";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import { useLibrary } from "@/state/library";

/** The small heart shown on a game tile. */
export function TileFavorite({ target }: { target: LaunchTarget }) {
  const { gameById, setFavorite } = useLibrary();
  const favorite = gameById.get(target.universeId)?.favorite ?? false;
  return (
    <button
      type="button"
      aria-pressed={favorite}
      aria-label={`${favorite ? "Remove" : "Add"} ${target.name} ${favorite ? "from" : "to"} favorites`}
      title={favorite ? "Remove from favorites" : "Add to favorites"}
      onClick={() => setFavorite(target, !favorite)}
      className={`absolute top-2 right-2 grid size-8 place-items-center rounded-full bg-black/55 text-white backdrop-blur-sm transition-opacity duration-(--fade) hover:bg-black/75 group-focus-within:opacity-100 group-hover:opacity-100 ${
        favorite ? "opacity-100" : "opacity-0"
      }`}
    >
      <Heart size={14} fill={favorite ? "currentColor" : "none"} aria-hidden="true" />
    </button>
  );
}

function CollectionsDialog({ target, onClose }: { target: LaunchTarget; onClose: () => void }) {
  const { library, gameById, setInCollection, createCollection } = useLibrary();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const selected = gameById.get(target.universeId)?.collectionIds ?? [];
  const collections = library?.collections ?? [];

  const create = async () => {
    const failure = await createCollection(name);
    setError(failure);
    if (!failure) setName("");
  };

  return (
    <Dialog
      title="Add to collection"
      onClose={onClose}
      actions={
        <Button variant="inset" onClick={onClose}>
          Done
        </Button>
      }
    >
      {collections.length === 0 && <p className="text-[13px]">You don't have any collections yet.</p>}
      <ul className="max-h-56 overflow-y-auto">
        {collections.map((collection) => (
          <li key={collection.id} className="flex items-center justify-between gap-4 py-1.5">
            <span className="truncate text-text">{collection.name}</span>
            <Switch
              checked={selected.includes(collection.id)}
              label={`${target.name} in ${collection.name}`}
              onChange={(included) => setInCollection(collection.id, target, included)}
            />
          </li>
        ))}
      </ul>
      <form
        className="mt-3 flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={40}
          aria-label="New collection name"
          placeholder="New collection"
          spellCheck={false}
          className="selectable h-9 min-w-0 flex-1 rounded-lg bg-hover px-3 text-[13px] text-text placeholder:text-faint"
        />
        <Button type="submit" variant="inset" disabled={name.trim() === ""}>
          Create
        </Button>
      </form>
      {error && (
        <p role="alert" className="mt-2 text-[13px] text-danger">
          {error}
        </p>
      )}
    </Dialog>
  );
}

/** Favorite, pin and collection buttons for a game's detail page. */
export function SaveControls({ target }: { target: LaunchTarget }) {
  const { gameById, setFavorite, setPinned } = useLibrary();
  const [picking, setPicking] = useState(false);
  const saved = gameById.get(target.universeId);
  const favorite = saved?.favorite ?? false;
  const pinned = saved?.pinned ?? false;

  return (
    <div className="mt-3 flex flex-wrap gap-2">
      <Button aria-pressed={favorite} onClick={() => setFavorite(target, !favorite)}>
        <Heart size={15} fill={favorite ? "currentColor" : "none"} aria-hidden="true" />
        {favorite ? "Favorited" : "Favorite"}
      </Button>
      <Button aria-pressed={pinned} onClick={() => setPinned(target, !pinned)}>
        <Pin size={15} fill={pinned ? "currentColor" : "none"} aria-hidden="true" />
        {pinned ? "Pinned" : "Pin"}
      </Button>
      <Button onClick={() => setPicking(true)}>
        <FolderPlus size={15} aria-hidden="true" />
        Collections
      </Button>
      {picking && <CollectionsDialog target={target} onClose={() => setPicking(false)} />}
    </div>
  );
}
