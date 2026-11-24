import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

export interface AppNotification {
  id: number;
  tone: "info" | "error";
  title: string;
  body?: string;
  at: Date;
}

interface NotificationsValue {
  items: readonly AppNotification[];
  unread: number;
  notify: (notification: Pick<AppNotification, "tone" | "title" | "body">) => void;
  markRead: () => void;
  clear: () => void;
}

const MAX_ITEMS = 50;
const NotificationsContext = createContext<NotificationsValue | null>(null);
let nextId = 1;

export function NotificationsProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<AppNotification[]>([]);
  const [unread, setUnread] = useState(0);

  const notify = useCallback<NotificationsValue["notify"]>((notification) => {
    setItems((current) =>
      [{ ...notification, id: nextId++, at: new Date() }, ...current].slice(0, MAX_ITEMS),
    );
    setUnread((count) => Math.min(count + 1, MAX_ITEMS));
  }, []);

  const markRead = useCallback(() => setUnread(0), []);
  const clear = useCallback(() => {
    setItems([]);
    setUnread(0);
  }, []);

  const value = useMemo(
    () => ({ items, unread, notify, markRead, clear }),
    [items, unread, notify, markRead, clear],
  );
  return <NotificationsContext.Provider value={value}>{children}</NotificationsContext.Provider>;
}

export function useNotifications(): NotificationsValue {
  const value = useContext(NotificationsContext);
  if (!value) throw new Error("useNotifications must be used inside NotificationsProvider");
  return value;
}
