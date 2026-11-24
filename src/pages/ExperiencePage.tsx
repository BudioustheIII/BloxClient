import { ArrowLeft, BadgeCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { LoadError } from "@/components/experience/LoadError";
import { PlayButton } from "@/components/experience/PlayButton";
import { SaveControls } from "@/components/experience/SaveControls";
import { Thumbnail } from "@/components/experience/Thumbnail";
import type { ExperienceDetail } from "@/lib/bindings/ExperienceDetail";
import { formatCount, ratingPercent, toLaunchTarget } from "@/lib/experience";
import { api, IpcFailure } from "@/lib/ipc";
import { useResource } from "@/lib/useResource";

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

function formatDate(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : dateFormat.format(date);
}

function count(value: number | null): string | null {
  return value === null ? null : formatCount(value);
}

function Stats({ detail }: { detail: ExperienceDetail }) {
  const { card } = detail;
  const rating = ratingPercent(card.upVotes, card.downVotes);
  const entries: readonly (readonly [string, string | null])[] = [
    ["Playing now", count(card.playing)],
    ["Visits", count(detail.visits)],
    ["Favorites", count(detail.favorites)],
    ["Positive votes", rating === null ? null : `${rating}%`],
    ["Server size", detail.maxPlayers === null ? null : String(detail.maxPlayers)],
    ["Genre", card.genre],
    ["Updated", formatDate(detail.updated)],
    ["Created", formatDate(detail.created)],
  ];
  return (
    <dl className="grid grid-cols-2 gap-x-6 gap-y-4 lg:grid-cols-4">
      {entries.flatMap(([label, value]) =>
        value === null
          ? []
          : [
              <div key={label}>
                <dt className="text-[13px] text-muted">{label}</dt>
                <dd className="font-display text-lg font-semibold">{value}</dd>
              </div>,
            ],
      )}
    </dl>
  );
}

function Gallery({ detail }: { detail: ExperienceDetail }) {
  const [index, setIndex] = useState(0);
  useEffect(() => setIndex(0), [detail.card.universeId]);
  const { images, card } = detail;
  const current = images[index] ?? null;

  return (
    <div>
      <Thumbnail url={current} name={card.name} eager className="aspect-video w-full rounded-xl" />
      {images.length > 1 && (
        <div className="mt-2 flex gap-2 overflow-x-auto pb-1" role="group" aria-label="Screenshots">
          {images.map((url, imageIndex) => (
            <button
              key={url}
              type="button"
              aria-label={`Show image ${imageIndex + 1} of ${images.length}`}
              aria-pressed={imageIndex === index}
              onClick={() => setIndex(imageIndex)}
              className={`w-24 shrink-0 overflow-hidden rounded-md transition-opacity duration-(--fade) ${
                imageIndex === index ? "opacity-100 ring-2 ring-text" : "opacity-60 hover:opacity-100"
              }`}
            >
              <Thumbnail url={url} name={card.name} className="aspect-video w-full" />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default function ExperiencePage() {
  const navigate = useNavigate();
  const { universeId } = useParams();
  const id = Number(universeId);
  const validId = Number.isSafeInteger(id) && id > 0;

  const resource = useResource(
    () =>
      validId
        ? api.experience(id)
        : Promise.reject(new IpcFailure("roblox_not_found", "This experience doesn't exist or isn't available.")),
    universeId ?? "",
  );
  const detail = resource.data;

  return (
    <div className="mx-auto max-w-6xl px-8 py-6">
      <button
        type="button"
        onClick={() => void navigate(-1)}
        className="mb-4 inline-flex h-8 items-center gap-1.5 rounded-md pr-2.5 pl-1.5 text-muted transition-colors duration-(--fade) hover:bg-raised hover:text-text"
      >
        <ArrowLeft size={16} aria-hidden="true" />
        Back
      </button>

      {resource.loading && !detail && (
        <div aria-busy="true" aria-label="Loading game" className="aspect-video max-w-3xl rounded-xl bg-raised/60" />
      )}
      {resource.error && <LoadError title="Game didn't load" message={resource.error} onRetry={resource.reload} />}

      {detail && (
        <article>
          <div className="grid gap-8 lg:grid-cols-[minmax(0,3fr)_minmax(16rem,2fr)]">
            <Gallery detail={detail} />
            <div>
              <h1 className="font-display text-4xl leading-[1.08] font-bold tracking-tight text-balance">
                {detail.card.name}
              </h1>
              {detail.card.creatorName && (
                <p className="mt-2 inline-flex items-center gap-1 text-muted">
                  By {detail.card.creatorName}
                  {detail.card.creatorVerified && <BadgeCheck size={15} aria-label="Verified" />}
                </p>
              )}
              <div className="mt-5">
                <PlayButton target={toLaunchTarget(detail.card)} large />
              </div>
              <SaveControls target={toLaunchTarget(detail.card)} />
              <p className="mt-3 text-xs text-faint">Opens in the official Roblox Player.</p>
            </div>
          </div>

          <div className="mt-8">
            <Stats detail={detail} />
          </div>

          {detail.description && (
            <section className="mt-8">
              <h2 className="mb-2 font-display text-xl font-semibold">About</h2>
              <p className="selectable max-w-[72ch] leading-relaxed wrap-break-word whitespace-pre-line text-muted">
                {detail.description}
              </p>
            </section>
          )}
        </article>
      )}
    </div>
  );
}
