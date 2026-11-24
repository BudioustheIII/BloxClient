import { CircleCheck, Pencil, Search, Trash2, TriangleAlert } from "lucide-react";
import { useMemo, useState } from "react";
import { Button, Switch } from "@/components/ui/controls";
import type { FlagView } from "@/lib/bindings/FlagView";
import { filterFlags } from "@/lib/fflags";

const MAX_VISIBLE = 200;
const ICON_BUTTON =
  "grid size-8 place-items-center rounded-md text-muted transition-colors duration-(--fade) hover:bg-raised hover:text-text";

function FlagRow({
  flag,
  readOnly,
  onToggle,
  onEdit,
  onRemove,
}: {
  flag: FlagView;
  readOnly: boolean;
  onToggle: (enabled: boolean) => void;
  onEdit: () => void;
  onRemove: () => void;
}) {
  return (
    <li className={`py-2.5 ${flag.enabled ? "" : "opacity-60"}`}>
      <div className="flex items-center gap-3">
        {!readOnly && <Switch checked={flag.enabled} label={`Enable ${flag.name}`} onChange={onToggle} />}
        <div className="min-w-0 flex-1">
          <p className="selectable truncate font-mono text-[13px]" title={flag.name}>
            {flag.name}
          </p>
          <p className="selectable truncate text-xs text-muted" title={flag.value}>
            {flag.value === "" ? "Empty" : flag.value} <span className="text-faint">({flag.kind})</span>
          </p>
        </div>
        {flag.listed && (
          <span
            className="inline-flex shrink-0 items-center gap-1 text-xs text-muted"
            title="On the allowlist Roblox published in September 2025. Roblox can change it at any time."
          >
            <CircleCheck size={13} aria-hidden="true" />
            Published list
          </span>
        )}
        {!readOnly && (
          <div className="flex shrink-0">
            <button type="button" aria-label={`Edit ${flag.name}`} className={ICON_BUTTON} onClick={onEdit}>
              <Pencil size={14} aria-hidden="true" />
            </button>
            <button type="button" aria-label={`Remove ${flag.name}`} className={ICON_BUTTON} onClick={onRemove}>
              <Trash2 size={14} aria-hidden="true" />
            </button>
          </div>
        )}
      </div>
      {flag.warnings.map((warning) => (
        <p key={warning} className="mt-1 flex items-start gap-1.5 text-xs text-danger">
          <TriangleAlert size={12} className="mt-0.5 shrink-0" aria-hidden="true" />
          {warning}
        </p>
      ))}
    </li>
  );
}

export function FlagList({
  flags,
  readOnly,
  onAdd,
  onToggle,
  onEdit,
  onRemove,
}: {
  flags: readonly FlagView[];
  readOnly: boolean;
  onAdd: () => void;
  onToggle: (flag: FlagView, enabled: boolean) => void;
  onEdit: (flag: FlagView) => void;
  onRemove: (flag: FlagView) => void;
}) {
  const [query, setQuery] = useState("");
  const matches = useMemo(() => filterFlags(flags, query), [flags, query]);
  const visible = matches.slice(0, MAX_VISIBLE);

  return (
    <>
      <div className="mt-2 flex gap-2">
        <div className="relative min-w-0 flex-1">
          <Search size={15} aria-hidden="true" className="absolute top-1/2 left-3 -translate-y-1/2 text-faint" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            aria-label="Search flags"
            placeholder="Search flags"
            spellCheck={false}
            className="selectable h-9 w-full rounded-lg bg-raised pr-3 pl-9 text-[13px] text-text placeholder:text-faint"
          />
        </div>
        {!readOnly && (
          <Button variant="primary" onClick={onAdd}>
            Add flag
          </Button>
        )}
      </div>

      {flags.length === 0 && (
        <p className="mt-4 text-[13px] text-muted">
          {readOnly ? "This preset sets no flags, so Roblox uses its own defaults." : "No flags in this profile yet."}
        </p>
      )}
      {flags.length > 0 && matches.length === 0 && (
        <p className="mt-4 text-[13px] text-muted">No flags match that search.</p>
      )}
      <ul className="mt-2">
        {visible.map((flag) => (
          <FlagRow
            key={flag.name}
            flag={flag}
            readOnly={readOnly}
            onToggle={(enabled) => onToggle(flag, enabled)}
            onEdit={() => onEdit(flag)}
            onRemove={() => onRemove(flag)}
          />
        ))}
      </ul>
      {matches.length > MAX_VISIBLE && (
        <p className="mt-2 text-xs text-faint">
          Showing the first {MAX_VISIBLE} of {matches.length}. Search to narrow the list.
        </p>
      )}
    </>
  );
}
