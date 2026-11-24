import { lazy, useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router";
import { AppShell } from "@/components/shell/AppShell";
import { api } from "@/lib/ipc";
import type { Bootstrap } from "@/lib/bindings/Bootstrap";
import { AuthProvider } from "@/state/auth";
import { DeploymentsProvider } from "@/state/deployments";
import { NotificationsProvider } from "@/state/notifications";
import { PlayProvider } from "@/state/play";
import { LibraryProvider } from "@/state/library";
import { SettingsProvider, useSettings } from "@/state/settings";

const AccountsPage = lazy(() => import("@/pages/AccountsPage"));
const DiscoverPage = lazy(() => import("@/pages/DiscoverPage"));
const ExperiencePage = lazy(() => import("@/pages/ExperiencePage"));
const FavoritesPage = lazy(() => import("@/pages/FavoritesPage"));
const FriendsPage = lazy(() => import("@/pages/FriendsPage"));
const HomePage = lazy(() => import("@/pages/HomePage"));
const LibraryPage = lazy(() => import("@/pages/LibraryPage"));
const PerformancePage = lazy(() => import("@/pages/PerformancePage"));
const SettingsPage = lazy(() => import("@/pages/settings/SettingsPage"));

function AppRoutes() {
  const { settings } = useSettings();
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to={`/${settings.general.defaultPage}`} replace />} />
        <Route path="home" element={<HomePage />} />
        <Route path="discover" element={<DiscoverPage />} />
        <Route path="experience/:universeId" element={<ExperiencePage />} />
        <Route path="accounts" element={<AccountsPage />} />
        <Route path="library" element={<LibraryPage />} />
        <Route path="favorites" element={<FavoritesPage />} />
        <Route path="friends" element={<FriendsPage />} />
        <Route path="performance" element={<PerformancePage />} />
        <Route path="settings" element={<Navigate to="/settings/general" replace />} />
        <Route path="settings/:section" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/home" replace />} />
      </Route>
    </Routes>
  );
}

export function App({ bootstrap }: { bootstrap: Bootstrap }) {
  useEffect(() => {
    // Wait for a painted frame so the window never appears blank.
    const frame = requestAnimationFrame(() => void api.appReady());
    return () => cancelAnimationFrame(frame);
  }, []);

  return (
    <NotificationsProvider>
      <SettingsProvider initial={bootstrap.settings}>
        <DeploymentsProvider>
          <PlayProvider>
            <LibraryProvider>
              <AuthProvider>
                <HashRouter>
                  <AppRoutes />
                </HashRouter>
              </AuthProvider>
            </LibraryProvider>
          </PlayProvider>
        </DeploymentsProvider>
      </SettingsProvider>
    </NotificationsProvider>
  );
}
