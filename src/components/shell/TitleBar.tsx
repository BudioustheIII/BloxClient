import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, PanelLeft, Square, X } from "lucide-react";
import { useEffect, useState, type ButtonHTMLAttributes } from "react";
import { useSettings } from "@/state/settings";
import { Logo } from "./Logo";
import { NotificationCenter } from "./NotificationCenter";

const appWindow = getCurrentWindow();

function WindowButton({ className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={`grid h-10 w-11 place-items-center text-muted transition-colors duration-(--fade) hover:bg-hover hover:text-text focus-visible:-outline-offset-2 ${className}`}
      {...props}
    />
  );
}

export function TitleBar() {
  const { settings, update } = useSettings();
  const [maximized, setMaximized] = useState(false);
  const compact = settings.appearance.sidebar === "compact";

  useEffect(() => {
    let active = true;
    const sync = () => {
      void appWindow.isMaximized().then((value) => {
        if (active) setMaximized(value);
      });
    };
    sync();
    const unlisten = appWindow.onResized(sync);
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return (
    <header data-tauri-drag-region className="flex h-10 items-center bg-shell">
      <button
        type="button"
        aria-label={compact ? "Expand sidebar" : "Collapse sidebar"}
        aria-expanded={!compact}
        title={compact ? "Expand sidebar" : "Collapse sidebar"}
        onClick={() => void update({ appearance: { sidebar: compact ? "expanded" : "compact" } })}
        className="ml-3 grid size-8 place-items-center rounded-md text-muted transition-colors duration-(--fade) hover:bg-hover hover:text-text"
      >
        <PanelLeft size={16} aria-hidden="true" />
      </button>
      <div data-tauri-drag-region className="ml-3 flex items-center gap-2">
        <Logo size={20} />
        <span data-tauri-drag-region className="font-display text-[15px] font-semibold tracking-tight">
          BloxClient
        </span>
      </div>
      <div data-tauri-drag-region className="h-full flex-1" />
      <NotificationCenter />
      <div className="ml-2 flex">
        <WindowButton aria-label="Minimize" onClick={() => void appWindow.minimize()}>
          <Minus size={15} aria-hidden="true" />
        </WindowButton>
        <WindowButton
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={() => void appWindow.toggleMaximize()}
        >
          {maximized ? (
            <Copy size={13} className="-scale-x-100" aria-hidden="true" />
          ) : (
            <Square size={12} aria-hidden="true" />
          )}
        </WindowButton>
        <WindowButton
          aria-label="Close"
          className="hover:bg-[#c42b1c] hover:text-white"
          onClick={() => void appWindow.close()}
        >
          <X size={16} aria-hidden="true" />
        </WindowButton>
      </div>
    </header>
  );
}
