import { ChevronLeft, ChevronRight } from "lucide-react";
import { Children, useRef, type ReactNode } from "react";

/** A titled, horizontally scrolling row of game tiles. */
export function Shelf({ title, children }: { title: string; children: ReactNode }) {
  const track = useRef<HTMLDivElement>(null);

  const scroll = (direction: 1 | -1) => {
    const element = track.current;
    if (!element) return;
    const animated =
      document.documentElement.dataset.motion === "full" &&
      !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    element.scrollBy({ left: direction * element.clientWidth * 0.85, behavior: animated ? "smooth" : "auto" });
  };

  const arrow =
    "grid size-8 place-items-center rounded-md text-muted transition-colors duration-(--fade) hover:bg-raised hover:text-text";

  return (
    <section aria-label={title} className="mt-8">
      <div className="mb-3 flex items-center justify-between px-8">
        <h2 className="font-display text-xl font-semibold tracking-tight">{title}</h2>
        <div className="flex gap-1">
          <button type="button" aria-label={`Scroll ${title} left`} className={arrow} onClick={() => scroll(-1)}>
            <ChevronLeft size={18} aria-hidden="true" />
          </button>
          <button type="button" aria-label={`Scroll ${title} right`} className={arrow} onClick={() => scroll(1)}>
            <ChevronRight size={18} aria-hidden="true" />
          </button>
        </div>
      </div>
      <div ref={track} className="shelf-track flex gap-4 overflow-x-auto px-8 pb-1">
        {Children.map(children, (child) => (
          <div className="w-60 shrink-0">{child}</div>
        ))}
      </div>
    </section>
  );
}
