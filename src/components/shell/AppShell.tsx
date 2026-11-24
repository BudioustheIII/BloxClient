import { Suspense } from "react";
import { Outlet, useLocation } from "react-router";
import { WelcomeScreen } from "@/components/auth/WelcomeScreen";
import { AppBackdrop } from "./AppBackdrop";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";

export function AppShell() {
  const { pathname } = useLocation();
  const pageKey = pathname.split("/")[1];

  return (
    <div className="grid h-full grid-cols-[minmax(0,1fr)] grid-rows-[40px_minmax(0,1fr)] text-text">
      <AppBackdrop />
      <TitleBar />
      <div className="flex min-h-0">
        <Sidebar />
        <div className="relative flex min-h-0 min-w-0 flex-1">
          <span aria-hidden="true" className="shell-corner" />
          <main className="min-w-0 flex-1 overflow-y-auto rounded-tl-2xl bg-panel">
            <Suspense fallback={null}>
              <div key={pageKey} className="fade-in h-full">
                <Outlet />
              </div>
            </Suspense>
          </main>
        </div>
      </div>
      <WelcomeScreen />
    </div>
  );
}
