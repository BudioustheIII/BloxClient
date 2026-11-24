import type { ButtonHTMLAttributes, ReactNode } from "react";

const BUTTON_TONES = {
  secondary: "bg-raised text-text hover:bg-hover",
  /** For buttons placed on a raised panel. */
  inset: "bg-hover text-text hover:bg-line",
  primary: "bg-accent text-on-accent hover:opacity-90",
  danger: "bg-danger text-black hover:opacity-90",
} as const;

type ButtonVariant = keyof typeof BUTTON_TONES;

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  const tone = BUTTON_TONES[variant];
  return (
    <button
      type="button"
      className={`inline-flex h-9 items-center gap-2 rounded-lg px-3.5 text-[13px] font-medium transition-colors duration-(--fade) disabled:opacity-50 ${tone} ${className}`}
      {...props}
    />
  );
}

export function Switch({
  checked,
  label,
  onChange,
}: {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-10 shrink-0 rounded-full transition-colors duration-(--fade) ${
        checked ? "bg-accent" : "bg-hover"
      }`}
    >
      <span
        className={`absolute top-1 left-1 size-4 rounded-full transition-transform duration-(--move) ${
          checked ? "translate-x-4 bg-on-accent" : "bg-muted"
        }`}
      />
    </button>
  );
}

export function Segmented<T extends string | number>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex shrink-0 rounded-lg bg-raised p-0.5">
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(option.value)}
            className={`h-8 rounded-md px-3 text-[13px] transition-colors duration-(--fade) ${
              selected ? "bg-hover font-semibold text-text" : "text-muted hover:text-text"
            }`}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

export function Select<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <select
      aria-label={label}
      value={value}
      onChange={(event) => onChange(event.target.value as T)}
      className="h-9 shrink-0 rounded-lg bg-raised px-3 text-[13px] text-text hover:bg-hover"
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

export function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex min-h-14 items-center justify-between gap-8 py-3">
      <div className="min-w-0">
        <p className="font-medium">{title}</p>
        {description && <p className="max-w-md text-[13px] text-muted">{description}</p>}
      </div>
      {children}
    </div>
  );
}

export function SettingGroup({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-8">
      <h2 className="mb-1 font-display text-lg font-semibold">{title}</h2>
      {children}
    </section>
  );
}
