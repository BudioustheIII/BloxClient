import { Check } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Segmented, SettingGroup, SettingRow } from "@/components/ui/controls";
import type { MotionMode } from "@/lib/bindings/MotionMode";
import type { SidebarMode } from "@/lib/bindings/SidebarMode";
import type { Theme } from "@/lib/bindings/Theme";
import { readableInk } from "@/lib/color";
import { useSettings } from "@/state/settings";

const ACCENTS = [
  { name: "Chalk", value: "#E8E8E6" },
  { name: "Brick", value: "#F4C542" },
  { name: "Coral", value: "#FF7A66" },
  { name: "Ice", value: "#7CC7FF" },
  { name: "Mint", value: "#5FE0B0" },
  { name: "Orchid", value: "#C79BFF" },
] as const;

const THEMES: readonly { value: Theme; label: string }[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];
const SIDEBARS: readonly { value: SidebarMode; label: string }[] = [
  { value: "expanded", label: "Expanded" },
  { value: "compact", label: "Compact" },
];
const MOTION: readonly { value: MotionMode; label: string }[] = [
  { value: "full", label: "Full" },
  { value: "reduced", label: "Reduced" },
  { value: "off", label: "Off" },
];
const SCALES = [90, 100, 110, 125].map((value) => ({ value, label: `${value}%` }));

const CUSTOM_COLOR_DELAY_MS = 200;

function AccentPicker({ value, onChange }: { value: string; onChange: (accent: string) => void }) {
  const [custom, setCustom] = useState(value);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => setCustom(value), [value]);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  // The native picker fires continuously while dragging, so saves are debounced.
  const onCustom = (next: string) => {
    setCustom(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => onChange(next.toUpperCase()), CUSTOM_COLOR_DELAY_MS);
  };

  return (
    <div role="radiogroup" aria-label="Accent color" className="flex shrink-0 items-center gap-2">
      {ACCENTS.map((accent) => {
        const selected = accent.value.toLowerCase() === value.toLowerCase();
        return (
          <button
            key={accent.value}
            type="button"
            role="radio"
            aria-checked={selected}
            aria-label={accent.name}
            title={accent.name}
            onClick={() => onChange(accent.value)}
            style={{ backgroundColor: accent.value, color: readableInk(accent.value) }}
            className="grid size-7 place-items-center rounded-full ring-1 ring-line"
          >
            {selected && <Check size={14} strokeWidth={3} aria-hidden="true" />}
          </button>
        );
      })}
      <input
        type="color"
        aria-label="Custom accent color"
        title="Custom color"
        value={custom.toLowerCase()}
        onChange={(event) => onCustom(event.target.value)}
        className="ml-1 size-7 cursor-pointer rounded-md bg-transparent"
      />
    </div>
  );
}

export function AppearanceSection() {
  const { settings, update } = useSettings();
  const { appearance } = settings;

  return (
    <>
      <SettingGroup title="Color">
        <SettingRow title="Theme">
          <Segmented
            label="Theme"
            value={appearance.theme}
            options={THEMES}
            onChange={(theme) => void update({ appearance: { theme } })}
          />
        </SettingRow>
        <SettingRow title="Accent color">
          <AccentPicker
            value={appearance.accent}
            onChange={(accent) => void update({ appearance: { accent } })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Layout">
        <SettingRow title="Sidebar" description="Compact shows icons only.">
          <Segmented
            label="Sidebar"
            value={appearance.sidebar}
            options={SIDEBARS}
            onChange={(sidebar) => void update({ appearance: { sidebar } })}
          />
        </SettingRow>
        <SettingRow title="Interface scale">
          <Segmented
            label="Interface scale"
            value={appearance.uiScale}
            options={SCALES}
            onChange={(uiScale) => void update({ appearance: { uiScale } })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Motion">
        <SettingRow
          title="Animations"
          description="Reduced removes movement and keeps quick fades. Your Windows animation setting is also respected."
        >
          <Segmented
            label="Animations"
            value={appearance.motion}
            options={MOTION}
            onChange={(motion) => void update({ appearance: { motion } })}
          />
        </SettingRow>
      </SettingGroup>
    </>
  );
}
