import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { linePath, nearestIndex, xPercent, type ChartPoint } from "@/lib/performance";

const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "medium" });

/** A compact single-series line with a crosshair readout on hover or focus. */
export function LineChart({
  title,
  points,
  max,
  format,
}: {
  title: string;
  points: readonly ChartPoint[];
  max: number;
  format: (value: number) => string;
}) {
  const plot = useRef<HTMLDivElement>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const firstAt = points[0]?.at ?? 0;
  const lastAt = points[points.length - 1]?.at ?? 0;
  const path = linePath(points, max);
  const latest = [...points].reverse().find((point) => point.value !== null);
  const active = hovered !== null ? points[hovered] : undefined;
  const activeX = active ? xPercent(active.at, firstAt, lastAt) : 0;
  const scale = max > 0 ? max : 1;
  const endY = latest?.value !== null && latest ? 100 - (Math.min(latest.value!, scale) / scale) * 100 : 0;

  const onMove = (event: PointerEvent<HTMLDivElement>) => {
    const box = plot.current?.getBoundingClientRect();
    if (!box || box.width === 0) return;
    const index = nearestIndex(points, (event.clientX - box.left) / box.width);
    setHovered(index >= 0 ? index : null);
  };

  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (points.length === 0) return;
    const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
    if (step === 0) return;
    event.preventDefault();
    setHovered((current) => Math.min(points.length - 1, Math.max(0, (current ?? points.length - 1) + step)));
  };

  return (
    <figure className="min-w-0 rounded-xl bg-raised/60 p-4">
      <figcaption className="flex items-baseline justify-between gap-3">
        <span className="text-[13px] text-muted">{title}</span>
        <span className="font-display text-xl font-semibold">{latest?.value != null ? format(latest.value) : "—"}</span>
      </figcaption>
      <div
        ref={plot}
        role="img"
        tabIndex={0}
        aria-label={`${title}: ${latest?.value != null ? format(latest.value) : "not available"}. Use left and right arrows to read earlier values.`}
        className="relative mt-3 h-20 touch-none rounded-sm"
        onPointerMove={onMove}
        onPointerLeave={() => setHovered(null)}
        onKeyDown={onKey}
        onBlur={() => setHovered(null)}
      >
        <div aria-hidden="true" className="absolute inset-x-0 top-0 border-t border-line" />
        <div aria-hidden="true" className="absolute inset-x-0 bottom-0 border-t border-line" />
        <svg
          aria-hidden="true"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          className="absolute inset-0 size-full overflow-visible text-accent"
        >
          <path
            d={path}
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            vectorEffect="non-scaling-stroke"
          />
        </svg>
        {latest && !active && (
          <span
            aria-hidden="true"
            className="absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent ring-2 ring-raised"
            style={{ left: `${xPercent(latest.at, firstAt, lastAt)}%`, top: `${endY}%` }}
          />
        )}
        {active && (
          <>
            <div aria-hidden="true" className="absolute inset-y-0 border-l border-muted" style={{ left: `${activeX}%` }} />
            <div
              role="status"
              className={`pointer-events-none absolute -top-2 z-10 rounded-md border border-line bg-surface px-2 py-1 text-xs whitespace-nowrap shadow-lg shadow-black/30 ${
                activeX > 60 ? "-translate-x-full" : ""
              }`}
              style={{ left: `${activeX}%` }}
            >
              <span className="font-semibold text-text">{active.value === null ? "Not available" : format(active.value)}</span>
              <span className="ml-2 text-muted">{timeFormat.format(new Date(active.at))}</span>
            </div>
          </>
        )}
      </div>
    </figure>
  );
}
