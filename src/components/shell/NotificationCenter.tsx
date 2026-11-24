import { Bell } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useNotifications } from "@/state/notifications";

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

export function NotificationCenter() {
  const { items, unread, markRead, clear } = useNotifications();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    markRead();
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, items, markRead]);

  const label = unread > 0 ? `Notifications, ${unread} unread` : "Notifications";

  return (
    <div ref={rootRef} className="relative">
      <button
        ref={buttonRef}
        type="button"
        aria-label={label}
        title="Notifications"
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen((value) => !value)}
        className="relative grid size-8 place-items-center rounded-md text-muted transition-colors duration-(--fade) hover:bg-hover hover:text-text"
      >
        <Bell size={16} aria-hidden="true" />
        {unread > 0 && <span className="absolute top-1.5 right-1.5 size-2 rounded-full bg-accent" />}
      </button>

      {open && (
        <div
          role="dialog"
          aria-label="Notifications"
          className="fade-in absolute top-10 right-0 z-50 w-80 overflow-hidden rounded-xl border border-line bg-raised shadow-2xl shadow-black/50"
        >
          <div className="flex h-11 items-center justify-between px-4">
            <h2 className="font-display text-[15px] font-semibold">Notifications</h2>
            {items.length > 0 && (
              <button
                type="button"
                onClick={clear}
                className="rounded px-1.5 py-0.5 text-[13px] text-muted hover:text-text"
              >
                Clear all
              </button>
            )}
          </div>
          {items.length === 0 ? (
            <p className="px-4 pb-5 text-[13px] text-muted">
              Errors and finished tasks will appear here.
            </p>
          ) : (
            <ul className="max-h-96 overflow-y-auto pb-2">
              {items.map((item) => (
                <li key={item.id} className="px-4 py-2.5">
                  <div className="flex items-baseline justify-between gap-3">
                    <p className={`font-medium ${item.tone === "error" ? "text-danger" : ""}`}>
                      {item.title}
                    </p>
                    <time className="shrink-0 text-xs text-faint">{timeFormat.format(item.at)}</time>
                  </div>
                  {item.body && <p className="selectable text-[13px] text-muted">{item.body}</p>}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
