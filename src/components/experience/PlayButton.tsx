import { Play } from "lucide-react";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import { usePlay } from "@/state/play";

function useButtonState(target: LaunchTarget) {
  const { session, requesting, play } = usePlay();
  const isThis = session?.universeId === target.universeId;
  const label = isThis ? (session?.stage === "running" ? "Playing" : "Starting") : "Play";
  const disabled = session !== null || requesting;
  const title = !isThis && session ? `${session.name} is open` : undefined;
  return { label, disabled, title, onClick: () => play(target) };
}

export function PlayButton({ target, large = false }: { target: LaunchTarget; large?: boolean }) {
  const { label, disabled, title, onClick } = useButtonState(target);
  return (
    <button
      type="button"
      disabled={disabled}
      title={title}
      aria-label={label === "Play" ? `Play ${target.name}` : `${label} ${target.name}`}
      onClick={onClick}
      className={`inline-flex items-center justify-center gap-2 rounded-lg bg-accent font-semibold text-on-accent transition-opacity duration-(--fade) hover:opacity-90 disabled:opacity-60 ${
        large ? "h-12 min-w-40 px-8 text-base" : "h-10 min-w-28 px-5"
      }`}
    >
      <Play size={large ? 18 : 15} fill="currentColor" aria-hidden="true" />
      {label}
    </button>
  );
}

/** The small button shown on a game tile. */
export function QuickPlayButton({ target }: { target: LaunchTarget }) {
  const { label, disabled, title, onClick } = useButtonState(target);
  return (
    <button
      type="button"
      disabled={disabled}
      title={title ?? label}
      aria-label={`${label} ${target.name}`}
      onClick={onClick}
      className="absolute right-2 bottom-2 grid size-9 place-items-center rounded-full bg-accent text-on-accent opacity-0 shadow-lg shadow-black/40 transition-opacity duration-(--fade) group-focus-within:opacity-100 group-hover:opacity-100 disabled:opacity-0"
    >
      <Play size={15} fill="currentColor" aria-hidden="true" />
    </button>
  );
}
