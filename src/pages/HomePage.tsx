import { BadgeCheck, ThumbsUp, Users } from "lucide-react";
import { Link } from "react-router";
import { LoadError } from "@/components/experience/LoadError";
import { PlayButton } from "@/components/experience/PlayButton";
import { Shelf } from "@/components/experience/Shelf";
import { ExperienceTile, Tile } from "@/components/experience/ExperienceTile";
import { Thumbnail } from "@/components/experience/Thumbnail";
import { PersonCard } from "@/components/people/PersonCard";
import type { ExperienceCard } from "@/lib/bindings/ExperienceCard";
import { formatCount, formatRelativeTime, ratingPercent, savedTarget, toLaunchTarget } from "@/lib/experience";
import { api } from "@/lib/ipc";
import { isActivePresence } from "@/lib/performance";
import { usePeople } from "@/lib/usePeople";
import { useResource } from "@/lib/useResource";
import { useLibrary } from "@/state/library";
import { usePlay } from "@/state/play";

const HOME_ROWS = 4;
const HOME_PEOPLE = 6;
const HOME_PRESENCE_MS = 120_000;

function Featured({ card, rowTitle }: { card: ExperienceCard; rowTitle: string }) {
  const rating = ratingPercent(card.upVotes, card.downVotes);
  return (
    <section aria-label={`Featured from ${rowTitle}`} className="relative h-[26rem] overflow-hidden rounded-tl-2xl">
      <div className="hero-fade absolute inset-0">
        <Thumbnail url={card.thumbnailUrl} name={card.name} eager className="absolute inset-0 size-full" />
        <div className="absolute inset-0 bg-linear-to-t from-surface via-surface/55 to-transparent" />
      </div>
      <div className="absolute inset-x-0 bottom-0 px-8 pb-6">
        <p className="mb-1 text-[13px] font-medium text-text/80">{rowTitle}</p>
        <h1 className="max-w-3xl font-display text-5xl leading-[1.05] font-bold tracking-tight text-balance">
          {card.name}
        </h1>
        <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[13px] text-text/80">
          {card.creatorName && (
            <span className="inline-flex items-center gap-1">
              {card.creatorName}
              {card.creatorVerified && <BadgeCheck size={14} aria-label="Verified" />}
            </span>
          )}
          {card.playing !== null && (
            <span className="inline-flex items-center gap-1">
              <Users size={14} aria-hidden="true" />
              {formatCount(card.playing)} playing
            </span>
          )}
          {rating !== null && (
            <span className="inline-flex items-center gap-1">
              <ThumbsUp size={14} aria-hidden="true" />
              {rating}%
            </span>
          )}
        </div>
        <div className="mt-4 flex items-center gap-2">
          <PlayButton target={toLaunchTarget(card)} large />
          <Link
            to={`/experience/${card.universeId}`}
            className="inline-flex h-12 items-center rounded-lg bg-black/45 px-6 font-medium text-white backdrop-blur-sm transition-colors duration-(--fade) hover:bg-black/65"
          >
            View details
          </Link>
        </div>
      </div>
    </section>
  );
}

function ContinuePlaying() {
  const { recent } = usePlay();
  if (recent.length === 0) return null;
  return (
    <Shelf title="Continue playing">
      {recent.map((item) => (
        <Tile
          key={item.universeId}
          target={{
            universeId: item.universeId,
            placeId: item.placeId,
            name: item.name,
            creatorName: item.creatorName,
            thumbnailUrl: item.thumbnailUrl,
          }}
          subtitle={item.creatorName}
          meta={<span>{formatRelativeTime(item.lastPlayedAt)}</span>}
        />
      ))}
    </Shelf>
  );
}

function HomeFavorites() {
  const { library } = useLibrary();
  const games = (library?.games ?? [])
    .filter((game) => game.favorite || game.pinned)
    .sort((a, b) => Number(b.pinned) - Number(a.pinned));
  if (games.length === 0) return null;
  return (
    <Shelf title="Your games">
      {games.map((game) => (
        <Tile
          key={game.universeId}
          target={savedTarget(game)}
          subtitle={game.creatorName}
          meta={<span>{game.pinned ? "Pinned" : "Favorite"}</span>}
        />
      ))}
    </Shelf>
  );
}

function HomePeople() {
  const { overview } = usePeople(HOME_PRESENCE_MS);
  if (!overview?.activeId || overview.people.length === 0) return null;
  const people = [...overview.people]
    .sort((a, b) => Number(isActivePresence(b.presence)) - Number(isActivePresence(a.presence)))
    .slice(0, HOME_PEOPLE);
  return (
    <section aria-label="Friends" className="mt-8 px-8">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="font-display text-xl font-semibold tracking-tight">Friends</h2>
        <Link to="/friends" className="text-[13px] text-muted hover:text-text">
          View all
        </Link>
      </div>
      <ul className="grid gap-x-6 md:grid-cols-2">
        {people.map((person) => (
          <PersonCard key={person.userId} person={person} />
        ))}
      </ul>
    </section>
  );
}

function HomeSkeleton() {
  return (
    <div aria-busy="true" aria-label="Loading games">
      <div className="h-[26rem] rounded-tl-2xl bg-raised/50" />
      <div className="mt-8 flex gap-4 overflow-hidden px-8">
        {Array.from({ length: 6 }, (_, index) => (
          <div key={index} className="aspect-video w-60 shrink-0 rounded-lg bg-raised/60" />
        ))}
      </div>
    </div>
  );
}

export default function HomePage() {
  const feed = useResource(() => api.discoverFeed(), "home");
  const rows = feed.data?.rows ?? [];
  const firstRow = rows[0];
  const featured = firstRow?.experiences[0];

  return (
    <div className="pb-10">
      {feed.loading && !feed.data && <HomeSkeleton />}
      {feed.error && <LoadError title="Games didn't load" message={feed.error} onRetry={feed.reload} />}
      {featured && firstRow && <Featured card={featured} rowTitle={firstRow.title} />}
      <ContinuePlaying />
      <HomeFavorites />
      <HomePeople />
      {rows.slice(0, HOME_ROWS).map((row) => (
        <Shelf key={row.id} title={row.title}>
          {row.experiences.map((card) => (
            <ExperienceTile key={card.universeId} card={card} />
          ))}
        </Shelf>
      ))}
    </div>
  );
}
