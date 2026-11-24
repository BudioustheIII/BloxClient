import type { ComponentType } from "react";
import { Navigate, NavLink, useParams } from "react-router";
import { AdvancedSection } from "./AdvancedSection";
import { AppearanceSection } from "./AppearanceSection";
import { GeneralSection } from "./GeneralSection";
import { FFlagsSection } from "./fflags/FFlagsSection";
import { PrivacySection } from "./PrivacySection";
import { RobloxSection } from "./RobloxSection";

const SECTIONS: readonly { id: string; label: string; component: ComponentType }[] = [
  { id: "general", label: "General", component: GeneralSection },
  { id: "appearance", label: "Appearance", component: AppearanceSection },
  { id: "roblox", label: "Roblox", component: RobloxSection },
  { id: "fflags", label: "FFlags", component: FFlagsSection },
  { id: "privacy", label: "Privacy", component: PrivacySection },
  { id: "advanced", label: "Advanced", component: AdvancedSection },
];

export default function SettingsPage() {
  const { section } = useParams();
  const active = SECTIONS.find((item) => item.id === section);
  if (!active) return <Navigate to="/settings/general" replace />;
  const Section = active.component;

  return (
    <div className="flex min-h-full gap-10 px-10 py-9">
      <div className="w-40 shrink-0">
        <h1 className="mb-5 font-display text-3xl font-bold tracking-tight">Settings</h1>
        <nav aria-label="Settings sections">
          <ul className="space-y-0.5">
            {SECTIONS.map((item) => (
              <li key={item.id}>
                <NavLink
                  to={`/settings/${item.id}`}
                  className={({ isActive }) =>
                    `block rounded-md px-2.5 py-1.5 transition-colors duration-(--fade) ${
                      isActive ? "bg-raised font-semibold text-text" : "text-muted hover:text-text"
                    }`
                  }
                >
                  {item.label}
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>
      </div>
      <div key={active.id} className="fade-in max-w-2xl min-w-0 flex-1 pt-1">
        <Section />
      </div>
    </div>
  );
}
