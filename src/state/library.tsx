import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { LaunchTarget } from "@/lib/bindings/LaunchTarget";
import type { LibraryGame } from "@/lib/bindings/LibraryGame";
import type { LibraryOverview } from "@/lib/bindings/LibraryOverview";
import { savedTarget } from "@/lib/experience";
import { api, failureMessage } from "@/lib/ipc";
import { useNotifications } from "./notifications";

interface LibraryValue {
  library: LibraryOverview | null;
  gameById: ReadonlyMap<number, LibraryGame>;
  setFavorite: (target: LaunchTarget, favorite: boolean) => void;
  setPinned: (target: LaunchTarget, pinned: boolean) => void;
  setInCollection: (id: string, target: LaunchTarget, included: boolean) => void;
  /** These return an error message for showing inside a dialog. */
  createCollection: (name: string) => Promise<string | null>;
  renameCollection: (id: string, name: string) => Promise<string | null>;
  deleteCollection: (id: string) => void;
  clear: () => Promise<void>;
}

const LibraryContext = createContext<LibraryValue | null>(null);

export function LibraryProvider({ children }: { children: ReactNode }) {
  const { notify } = useNotifications();
  const [library, setLibrary] = useState<LibraryOverview | null>(null);

  useEffect(() => {
    api
      .library()
      .then(setLibrary)
      .catch((error: unknown) =>
        notify({
          tone: "error",
          title: "Library not loaded",
          body: failureMessage(error, "Your saved games couldn't be read."),
        }),
      );
  }, [notify]);

  const run = useCallback(
    (action: () => Promise<LibraryOverview>, failureTitle: string) => {
      action()
        .then(setLibrary)
        .catch((error: unknown) =>
          notify({ tone: "error", title: failureTitle, body: failureMessage(error, "That change wasn't saved.") }),
        );
    },
    [notify],
  );

  const attempt = useCallback(async (action: () => Promise<LibraryOverview>) => {
    try {
      setLibrary(await action());
      return null;
    } catch (error) {
      return failureMessage(error, "That change wasn't saved.");
    }
  }, []);

  const value = useMemo<LibraryValue>(
    () => ({
      library,
      gameById: new Map((library?.games ?? []).map((game) => [game.universeId, game])),
      setFavorite: (target, favorite) =>
        run(() => api.setGameFavorite(savedTarget(target), favorite), "Favorite not saved"),
      setPinned: (target, pinned) => run(() => api.setGamePinned(savedTarget(target), pinned), "Pin not saved"),
      setInCollection: (id, target, included) =>
        run(() => api.setGameInCollection(id, savedTarget(target), included), "Collection not updated"),
      createCollection: (name) => attempt(() => api.createCollection(name)),
      renameCollection: (id, name) => attempt(() => api.renameCollection(id, name)),
      deleteCollection: (id) => run(() => api.deleteCollection(id), "Collection not deleted"),
      clear: async () => {
        const error = await attempt(() => api.clearLibrary());
        notify(
          error
            ? { tone: "error", title: "Library not cleared", body: error }
            : { tone: "info", title: "Library cleared" },
        );
      },
    }),
    [library, run, attempt, notify],
  );

  return <LibraryContext.Provider value={value}>{children}</LibraryContext.Provider>;
}

export function useLibrary(): LibraryValue {
  const value = useContext(LibraryContext);
  if (!value) throw new Error("useLibrary must be used inside LibraryProvider");
  return value;
}
