import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Thumbnail } from "@/components/experience/Thumbnail";
import { formatDuration } from "@/lib/experience";
import { usePlay } from "@/state/play";

const CLOCK_INTERVAL_MS = 30_000;

export function SessionIndicator({ compact }: { compact: boolean }) {
  const { session } = usePlay();
  const [now, setNow] = useState(() => Date.now());
  const running = session?.stage === "running";

  useEffect(() => {
    if (!running) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), CLOCK_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [running]);

  if (!session) return null;

  const status =
    running && session.runningSince !== null
      ? `Playing, ${formatDuration(now / 1000 - session.runningSince).toLowerCase()}`
      : "Starting Roblox";

  return (
    <Link
      to={`/experience/${session.universeId}`}
      aria-label={`${session.name}. ${status}`}
      title={compact ? `${session.name}: ${status}` : undefined}
      className={`fade-in flex items-center gap-2.5 rounded-lg py-1.5 transition-colors duration-(--fade) hover:bg-raised ${
        compact ? "mx-auto px-1.5" : "mx-2 px-2"
      }`}
    >
      <span className="relative shrink-0">
        <Thumbnail url={session.thumbnailUrl} name={session.name} className="size-8 rounded-md text-sm" />
        <span
          aria-hidden="true"
          className={`absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full border-2 border-bg ${
            running ? "bg-accent" : "bg-muted"
          }`}
        />
      </span>
      {!compact && (
        <span className="min-w-0 leading-tight">
          <span className="block truncate text-[13px] font-medium">{session.name}</span>
          <span className="block truncate text-xs text-faint">{status}</span>
        </span>
      )}
    </Link>
  );
}
