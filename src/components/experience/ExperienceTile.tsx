import { BadgeCheck, ThumbsUp, Users } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";
import type { ExperienceCard } from "@/lib/bindings/ExperienceCard";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import { formatCount, ratingPercent, toLaunchTarget } from "@/lib/experience";
import { QuickPlayButton } from "./PlayButton";
import { TileFavorite } from "./SaveControls";
import { Thumbnail } from "./Thumbnail";

export function Tile({
  target,
  subtitle,
  verified = false,
  meta,
}: {
  target: LaunchTarget;
  subtitle: string | null;
  verified?: boolean;
  meta: ReactNode;
}) {
  return (
    <article className="group relative min-w-0">
      <div className="relative">
        <Link to={`/experience/${target.universeId}`} aria-label={`Open ${target.name}`} className="block rounded-lg">
          <Thumbnail
            url={target.thumbnailUrl}
            name={target.name}
            className="aspect-video w-full rounded-lg transition-[filter] duration-(--fade) group-hover:brightness-110"
          />
        </Link>
        <TileFavorite target={target} />
        <QuickPlayButton target={target} />
      </div>
      <h3 className="mt-2 truncate font-medium" title={target.name}>
        {target.name}
      </h3>
      {subtitle && (
        <p className="flex items-center gap-1 text-[13px] text-muted">
          <span className="truncate">{subtitle}</span>
          {verified && <BadgeCheck size={13} className="shrink-0" aria-label="Verified" />}
        </p>
      )}
      <div className="mt-0.5 flex items-center gap-3 text-xs text-faint">{meta}</div>
    </article>
  );
}

export function ExperienceTile({ card }: { card: ExperienceCard }) {
  const rating = ratingPercent(card.upVotes, card.downVotes);
  return (
    <Tile
      target={toLaunchTarget(card)}
      subtitle={card.creatorName}
      verified={card.creatorVerified}
      meta={
        <>
          {card.playing !== null && (
            <span className="inline-flex items-center gap-1" title="Playing now">
              <Users size={12} aria-hidden="true" />
              {formatCount(card.playing)}
              <span className="sr-only">playing now</span>
            </span>
          )}
          {rating !== null && (
            <span className="inline-flex items-center gap-1" title="Positive votes">
              <ThumbsUp size={12} aria-hidden="true" />
              {rating}%<span className="sr-only">positive votes</span>
            </span>
          )}
        </>
      }
    />
  );
}
