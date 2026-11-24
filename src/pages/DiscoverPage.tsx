import { Search, X } from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ExperienceTile } from "@/components/experience/ExperienceTile";
import { LoadError } from "@/components/experience/LoadError";
import { Button, Select } from "@/components/ui/controls";
import type { ExperienceCard } from "@/lib/bindings/ExperienceCard";
import { sortExperiences, type ExperienceSort } from "@/lib/experience";
import { api, failureMessage } from "@/lib/ipc";
import { useResource } from "@/lib/useResource";

const SORT_OPTIONS: readonly { value: ExperienceSort; label: string }[] = [
  { value: "recommended", label: "Recommended" },
  { value: "players", label: "Most players" },
  { value: "rating", label: "Highest rated" },
  { value: "name", label: "Name" },
];
const ALL_GENRES = "all";

function useSearch(query: string) {
  const [results, setResults] = useState<ExperienceCard[]>([]);
  const [nextPage, setNextPage] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    setResults([]);
    setNextPage(null);
    setError(null);
    if (!query) {
      setLoading(false);
      return;
    }
    let active = true;
    setLoading(true);
    api
      .searchExperiences(query)
      .then((page) => {
        if (!active) return;
        setResults(page.results);
        setNextPage(page.nextPage);
      })
      .catch((reason: unknown) => {
        if (active) setError(failureMessage(reason, "Search didn't finish."));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [query, attempt]);

  const loadMore = async () => {
    if (!nextPage || loading) return;
    setLoading(true);
    try {
      const page = await api.searchExperiences(query, nextPage);
      setResults((current) => {
        const seen = new Set(current.map((card) => card.universeId));
        return [...current, ...page.results.filter((card) => !seen.has(card.universeId))];
      });
      setNextPage(page.nextPage);
    } catch (reason) {
      setError(failureMessage(reason, "More results didn't load."));
    } finally {
      setLoading(false);
    }
  };

  return { results, nextPage, loading, error, loadMore, retry: () => setAttempt((value) => value + 1) };
}

export default function DiscoverPage() {
  const feed = useResource(() => api.discoverFeed(), "discover");
  const [input, setInput] = useState("");
  const [query, setQuery] = useState("");
  const [rowId, setRowId] = useState<string | null>(null);
  const [sort, setSort] = useState<ExperienceSort>("recommended");
  const [genre, setGenre] = useState(ALL_GENRES);
  const search = useSearch(query);

  const rows = feed.data?.rows ?? [];
  const activeRow = rows.find((row) => row.id === rowId) ?? rows[0];
  const source = query ? search.results : (activeRow?.experiences ?? []);

  const genres = useMemo(
    () => [...new Set(source.flatMap((card) => (card.genre ? [card.genre] : [])))].sort(),
    [source],
  );
  const genreOptions = useMemo(
    () => [{ value: ALL_GENRES, label: "All genres" }, ...genres.map((value) => ({ value, label: value }))],
    [genres],
  );
  const activeGenre = genres.includes(genre) ? genre : ALL_GENRES;
  const visible = useMemo(
    () =>
      sortExperiences(
        activeGenre === ALL_GENRES ? source : source.filter((card) => card.genre === activeGenre),
        sort,
      ),
    [source, activeGenre, sort],
  );

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setQuery(input.trim());
  };
  const clear = () => {
    setInput("");
    setQuery("");
  };

  const loading = query ? search.loading && search.results.length === 0 : feed.loading && !feed.data;
  const error = query ? search.error : feed.error;
  const retry = query ? search.retry : feed.reload;

  return (
    <div className="px-8 py-8">
      <h1 className="font-display text-4xl font-bold tracking-tight">Discover</h1>

      <form onSubmit={submit} role="search" className="relative mt-5 max-w-2xl">
        <Search size={17} aria-hidden="true" className="absolute top-1/2 left-3.5 -translate-y-1/2 text-faint" />
        <input
          type="search"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          maxLength={100}
          aria-label="Search games"
          placeholder="Search games"
          spellCheck={false}
          className="selectable h-11 w-full rounded-lg bg-raised pr-10 pl-10 text-text placeholder:text-faint [&::-webkit-search-cancel-button]:hidden"
        />
        {input && (
          <button
            type="button"
            aria-label="Clear search"
            onClick={clear}
            className="absolute top-1/2 right-2 grid size-7 -translate-y-1/2 place-items-center rounded-md text-muted hover:bg-hover hover:text-text"
          >
            <X size={15} aria-hidden="true" />
          </button>
        )}
      </form>

      {!query && rows.length > 0 && (
        <div role="tablist" aria-label="Categories" className="mt-5 flex flex-wrap gap-1.5">
          {rows.map((row) => {
            const selected = row.id === activeRow?.id;
            return (
              <button
                key={row.id}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => setRowId(row.id)}
                className={`h-8 rounded-full px-3.5 text-[13px] transition-colors duration-(--fade) ${
                  selected ? "bg-accent font-semibold text-on-accent" : "bg-raised text-muted hover:text-text"
                }`}
              >
                {row.title}
              </button>
            );
          })}
        </div>
      )}

      <div className="mt-6 mb-4 flex items-center justify-between gap-4">
        <h2 aria-live="polite" className="min-w-0 truncate font-display text-xl font-semibold">
          {query ? `Results for "${query}"` : (activeRow?.title ?? "")}
        </h2>
        <div className="flex shrink-0 gap-2">
          <Select label="Filter by genre" value={activeGenre} options={genreOptions} onChange={setGenre} />
          <Select label="Sort games" value={sort} options={SORT_OPTIONS} onChange={setSort} />
        </div>
      </div>

      {error && <LoadError title="Games didn't load" message={error} onRetry={retry} />}
      {loading && <p className="text-muted">{query ? "Searching" : "Loading games"}</p>}
      {!loading && !error && query && visible.length === 0 && (
        <p className="text-muted">No games match that search. Try a different name.</p>
      )}

      <div className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-x-4 gap-y-6">
        {visible.map((card) => (
          <ExperienceTile key={card.universeId} card={card} />
        ))}
      </div>

      {query && search.nextPage && !search.error && (
        <div className="mt-8 flex justify-center">
          <Button disabled={search.loading} onClick={() => void search.loadMore()}>
            {search.loading ? "Loading" : "Load more"}
          </Button>
        </div>
      )}
    </div>
  );
}
