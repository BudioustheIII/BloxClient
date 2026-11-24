import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { AuthStatus } from "@/lib/bindings/AuthStatus";
import { api, failureMessage } from "@/lib/ipc";
import { useNotifications } from "./notifications";

interface AuthValue {
  status: AuthStatus | null;
  error: string | null;
  connect: () => Promise<boolean>;
  cancel: () => Promise<boolean>;
  signOut: () => Promise<boolean>;
  saveClientId: (clientId: string) => Promise<boolean>;
  dismissWelcome: () => Promise<boolean>;
  openDashboard: () => void;
}

const AuthContext = createContext<AuthValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const { notify } = useNotifications();
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const previousState = useRef<AuthStatus["state"] | null>(null);

  useEffect(() => {
    let active = true;
    api
      .authStatus()
      .then((value) => {
        if (!active) return;
        previousState.current = value.state;
        setStatus(value);
      })
      .catch((error: unknown) => {
        if (active) setActionError(failureMessage(error, "Your account status couldn't be read."));
      });

    const unlisten = listen<AuthStatus>("auth-changed", ({ payload }) => {
      if (payload.state === "connected" && previousState.current === "signingIn" && payload.account) {
        notify({ tone: "info", title: "Roblox account connected", body: `@${payload.account.username}` });
      }
      previousState.current = payload.state;
      setActionError(null);
      setStatus(payload);
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, [notify]);

  const run = useCallback(async (action: () => Promise<AuthStatus>) => {
    setActionError(null);
    try {
      const next = await action();
      previousState.current = next.state;
      setStatus(next);
      return true;
    } catch (error) {
      setActionError(failureMessage(error, "That action didn't finish."));
      return false;
    }
  }, []);

  const connect = useCallback(() => run(api.startSignIn), [run]);
  const cancel = useCallback(() => run(api.cancelSignIn), [run]);
  const dismissWelcome = useCallback(() => run(api.dismissWelcome), [run]);
  const saveClientId = useCallback((clientId: string) => run(() => api.setOAuthClientId(clientId)), [run]);
  const signOut = useCallback(async () => {
    const done = await run(api.signOut);
    if (done) notify({ tone: "info", title: "Logged out of BloxClient" });
    return done;
  }, [notify, run]);
  const openDashboard = useCallback(() => {
    api.openOAuthDashboard().catch((error: unknown) => {
      setActionError(failureMessage(error, "Creator Dashboard couldn't be opened."));
    });
  }, []);

  const error = actionError ?? status?.error ?? null;
  const value = useMemo(
    () => ({ status, error, connect, cancel, signOut, saveClientId, dismissWelcome, openDashboard }),
    [status, error, connect, cancel, signOut, saveClientId, dismissWelcome, openDashboard],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthValue {
  const value = useContext(AuthContext);
  if (!value) throw new Error("useAuth must be used inside AuthProvider");
  return value;
}
