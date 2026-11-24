import { UserRound } from "lucide-react";
import { NavLink, useLocation } from "react-router";
import { NAV_ITEMS } from "@/app/navigation";
import { Thumbnail } from "@/components/experience/Thumbnail";
import { useAuth } from "@/state/auth";
import { useSettings } from "@/state/settings";
import { SessionIndicator } from "./SessionIndicator";

const ROW_HEIGHT = 40;

export function Sidebar() {
  const { settings } = useSettings();
  const { status } = useAuth();
  const account = status?.account ?? null;
  const { pathname } = useLocation();
  const compact = settings.appearance.sidebar === "compact";
  const section = pathname.split("/")[1];
  const activeIndex = NAV_ITEMS.findIndex((item) => item.id === section);

  return (
    <aside className={`flex shrink-0 flex-col bg-shell pt-2 pb-3 ${compact ? "w-16" : "w-52"}`}>
      <nav aria-label="Main" className="relative">
        {activeIndex >= 0 && (
          <span
            aria-hidden="true"
            className="stud-marker"
            style={{ transform: `translateY(${activeIndex * ROW_HEIGHT}px)` }}
          />
        )}
        <ul>
          {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
            <li key={id}>
              <NavLink
                to={`/${id}`}
                aria-label={compact ? label : undefined}
                title={compact ? label : undefined}
                className={({ isActive }) =>
                  `flex h-10 items-center gap-2.5 pl-3.5 transition-colors duration-(--fade) focus-visible:-outline-offset-2 ${
                    isActive ? "font-semibold text-text" : "text-muted hover:text-text"
                  }`
                }
              >
                <span className="stud-socket" aria-hidden="true" />
                <Icon size={18} strokeWidth={1.75} aria-hidden="true" className="shrink-0" />
                {!compact && <span className="truncate">{label}</span>}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>

      <div className="mt-auto flex flex-col">
      <SessionIndicator compact={compact} />
      <NavLink
        to="/accounts"
        aria-label={account ? `${account.displayName}. Open Accounts.` : "No Roblox account connected. Open Accounts."}
        title={compact ? (account?.displayName ?? "No account connected") : undefined}
        className={`flex items-center gap-2.5 rounded-lg py-1.5 transition-colors duration-(--fade) hover:bg-raised ${
          compact ? "mx-auto px-1.5" : "mx-2 px-2"
        }`}
      >
        {account ? (
          <Thumbnail url={account.avatarUrl} name={account.displayName} className="size-8 shrink-0 rounded-full text-sm" />
        ) : (
          <span className="grid size-8 shrink-0 place-items-center rounded-full bg-raised text-muted">
            <UserRound size={16} aria-hidden="true" />
          </span>
        )}
        {!compact && (
          <span className="min-w-0 leading-tight">
            <span className="block truncate text-[13px] font-medium">{account?.displayName ?? "No account"}</span>
            <span className="block truncate text-xs text-faint">
              {account ? `@${account.username}` : "Not connected"}
            </span>
          </span>
        )}
      </NavLink>
      </div>
    </aside>
  );
}
