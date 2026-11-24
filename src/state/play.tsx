import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/controls";
import { Dialog } from "@/components/ui/Dialog";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import type { PlaySession } from "@/lib/bindings/PlaySession";
import type { RecentExperience } from "@/lib/bindings/RecentExperience";
import { api, failureMessage, IpcFailure } from "@/lib/ipc";
import { useNotifications } from "./notifications";

interface PlayValue {
  session: PlaySession | null;
  /** True between pressing Play and Roblox accepting the launch request. */
  requesting: boolean;
  recent: readonly RecentExperience[];
  play: (target: LaunchTarget) => void;
  clearRecent: () => Promise<void>;
}

type Prompt = { kind: "running"; target: LaunchTarget } | { kind: "error"; title: string; message: string };

const PlayContext = createContext<PlayValue | null>(null);

export function PlayProvider({ children }: { children: ReactNode }) {
  const { notify } = useNotifications();
  const [session, setSession] = useState<PlaySession | null>(null);
  const [requesting, setRequesting] = useState(false);
  const [recent, setRecent] = useState<RecentExperience[]>([]);
  const [prompt, setPrompt] = useState<Prompt | null>(null);

  const refreshRecent = useCallback(async () => {
    try {
      setRecent(await api.recentExperiences());
    } catch (error) {
      notify({
        tone: "error",
        title: "Play history not loaded",
        body: failureMessage(error, "Your recently played games couldn't be read."),
      });
    }
  }, [notify]);

  useEffect(() => {
    // Show saved history right away, then add anything new from Roblox's logs.
    void refreshRecent().then(() =>
      api
        .syncPlayHistory()
        .then(setRecent)
        .catch((error: unknown) => console.warn("Could not import Roblox play history", error)),
    );
    void api.playSession().then((current) => {
      setSession((existing) => existing ?? current);
    });
    const unlistenHistory = listen("play-history-changed", () => void refreshRecent());
    const unlisten = listen<PlaySession>("play-session", ({ payload }) => {
      if (payload.stage === "launching") {
        setSession(payload);
      } else if (payload.stage === "running") {
        setSession(payload);
        void refreshRecent();
      } else {
        setSession(null);
        void refreshRecent();
        if (payload.stage === "failed") {
          const message = payload.message ?? "Roblox didn't start.";
          notify({ tone: "error", title: `${payload.name} didn't start`, body: message });
          setPrompt({ kind: "error", title: "Roblox didn't start", message });
        }
      }
    });
    return () => {
      void unlisten.then((stop) => stop());
      void unlistenHistory.then((stop) => stop());
    };
  }, [notify, refreshRecent]);

  const launch = useCallback(async (target: LaunchTarget, force: boolean) => {
    setRequesting(true);
    setPrompt(null);
    try {
      await api.launchExperience(target, force);
    } catch (error) {
      if (error instanceof IpcFailure && error.code === "roblox_already_running") {
        setPrompt({ kind: "running", target });
      } else {
        setPrompt({
          kind: "error",
          title: "Game not launched",
          message: failureMessage(error, "This game couldn't be started."),
        });
      }
    } finally {
      setRequesting(false);
    }
  }, []);

  const play = useCallback((target: LaunchTarget) => void launch(target, false), [launch]);

  const clearRecent = useCallback(async () => {
    try {
      await api.clearRecentExperiences();
      setRecent([]);
      notify({ tone: "info", title: "Play history cleared" });
    } catch (error) {
      notify({
        tone: "error",
        title: "Play history not cleared",
        body: failureMessage(error, "Your play history couldn't be cleared."),
      });
    }
  }, [notify]);

  const value = useMemo(
    () => ({ session, requesting, recent, play, clearRecent }),
    [session, requesting, recent, play, clearRecent],
  );

  return (
    <PlayContext.Provider value={value}>
      {children}
      {prompt?.kind === "running" && (
        <Dialog
          title="Roblox is already running"
          onClose={() => setPrompt(null)}
          actions={
            <>
              <Button variant="inset" onClick={() => setPrompt(null)}>
                Stay in current game
              </Button>
              <Button variant="primary" onClick={() => void launch(prompt.target, true)}>
                Join {prompt.target.name.length > 24 ? "this game" : prompt.target.name}
              </Button>
            </>
          }
        >
          Joining this game may close the game you have open.
        </Dialog>
      )}
      {prompt?.kind === "error" && (
        <Dialog
          title={prompt.title}
          onClose={() => setPrompt(null)}
          actions={
            <Button variant="inset" onClick={() => setPrompt(null)}>
              Close
            </Button>
          }
        >
          {prompt.message}
        </Dialog>
      )}
    </PlayContext.Provider>
  );
}

export function usePlay(): PlayValue {
  const value = useContext(PlayContext);
  if (!value) throw new Error("usePlay must be used inside PlayProvider");
  return value;
}
