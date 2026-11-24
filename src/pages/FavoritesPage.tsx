import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import { Tile } from "@/components/experience/ExperienceTile";
import { Select } from "@/components/ui/controls";
import { formatRelativeTime, matchesGame, savedTarget } from "@/lib/experience";
import { useLibrary } from "@/state/library";

type FavoriteSort = "added" | "name";

const SORT_OPTIONS: readonly { value: FavoriteSort; label: string }[] = [
  { value: "added", label: "Recently added" },
  { value: "name", label: "Name" },
];

export default function FavoritesPage() {
  const { library } = useLibrary();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<FavoriteSort>("added");

  const favorites = useMemo(() => {
    const games = (library?.games ?? []).filter((game) => game.favorite);
    return sort === "name" ? [...games].sort((a, b) => a.name.localeCompare(b.name)) : games;
  }, [library, sort]);
  const visible = favorites.filter((game) => matchesGame(game, query));

  return (
    <div className="px-8 py-8">
      <h1 className="font-display text-4xl font-bold tracking-tight">Favorites</h1>
      <p className="mt-2 max-w-prose text-[13px] text-muted">
        Saved on this PC. These are separate from your Roblox account favorites, which need a signed-in session.
      </p>

      <div className="mt-5 flex flex-wrap items-center gap-2">
        <div className="relative w-full max-w-md">
          <Search size={16} aria-hidden="true" className="absolute top-1/2 left-3 -translate-y-1/2 text-faint" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            aria-label="Search favorites"
            placeholder="Search favorites"
            spellCheck={false}
            className="selectable h-10 w-full rounded-lg bg-raised pr-3 pl-9 text-text placeholder:text-faint"
          />
        </div>
        <Select label="Sort favorites" value={sort} options={SORT_OPTIONS} onChange={setSort} />
      </div>

      {favorites.length === 0 && (
        <p className="mt-8 max-w-prose text-muted">
          No favorites yet. Hover over any game and press the heart to save it here.
        </p>
      )}
      {favorites.length > 0 && visible.length === 0 && <p className="mt-8 text-muted">No favorites match that search.</p>}

      <div className="mt-6 grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-x-4 gap-y-6">
        {visible.map((game) => (
          <Tile
            key={game.universeId}
            target={savedTarget(game)}
            subtitle={game.creatorName}
            meta={<span>Added {formatRelativeTime(game.addedAt).toLowerCase()}</span>}
          />
        ))}
      </div>
    </div>
  );
}
