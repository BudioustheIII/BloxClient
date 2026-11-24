import { useState, type ChangeEvent } from "react";
import { Button, Segmented } from "@/components/ui/controls";
import { Dialog } from "@/components/ui/Dialog";
import type { FlagInput } from "@/lib/bindings/FlagInput";
import type { FlagKind } from "@/lib/bindings/FlagKind";
import type { FlagView } from "@/lib/bindings/FlagView";
import { guessFlagKind } from "@/lib/fflags";

const MAX_IMPORT_BYTES = 1_000_000;
const FIELD =
  "selectable mt-1 h-9 w-full rounded-lg bg-hover px-3 text-[13px] text-text placeholder:text-faint disabled:opacity-60";
const KIND_OPTIONS: readonly { value: FlagKind; label: string }[] = [
  { value: "boolean", label: "Boolean" },
  { value: "number", label: "Number" },
  { value: "string", label: "String" },
];
const BOOLEAN_OPTIONS = [
  { value: "true", label: "True" },
  { value: "false", label: "False" },
] as const;

function FormError({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <p role="alert" className="mt-3 text-[13px] text-danger">
      {message}
    </p>
  );
}

export function FlagDialog({
  flag,
  onSave,
  onClose,
}: {
  flag: FlagView | null;
  onSave: (input: FlagInput) => Promise<string | null>;
  onClose: () => void;
}) {
  const [name, setName] = useState(flag?.name ?? "");
  const [kind, setKind] = useState<FlagKind>(flag?.kind ?? "boolean");
  const [kindChosen, setKindChosen] = useState(flag !== null);
  const [value, setValue] = useState(flag?.value ?? "true");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const changeKind = (next: FlagKind) => {
    setKind(next);
    if (next === "boolean") setValue((current) => (current.toLowerCase() === "false" ? "false" : "true"));
    else if (kind === "boolean") setValue("");
  };

  const changeName = (next: string) => {
    setName(next);
    const guess = guessFlagKind(next);
    if (!kindChosen && guess && guess !== kind) changeKind(guess);
  };

  const save = async () => {
    setSaving(true);
    const failure = await onSave({ name, kind, value, enabled: flag?.enabled ?? true });
    setSaving(false);
    if (failure) setError(failure);
    else onClose();
  };

  return (
    <Dialog
      title={flag ? "Edit flag" : "Add flag"}
      onClose={onClose}
      actions={
        <>
          <Button variant="inset" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" disabled={saving || name.trim() === ""} onClick={() => void save()}>
            Save flag
          </Button>
        </>
      }
    >
      <label className="mt-2 block text-[13px]">
        Name
        <input
          value={name}
          onChange={(event) => changeName(event.target.value)}
          maxLength={200}
          spellCheck={false}
          autoComplete="off"
          placeholder="FIntExampleFlag"
          className={FIELD}
        />
      </label>
      <div className="mt-3 text-[13px]">
        <p className="mb-1">Type</p>
        <Segmented
          label="Value type"
          value={kind}
          options={KIND_OPTIONS}
          onChange={(next) => {
            setKindChosen(true);
            changeKind(next);
          }}
        />
      </div>
      <div className="mt-3 text-[13px]">
        {kind === "boolean" ? (
          <>
            <p className="mb-1">Value</p>
            <Segmented
              label="Value"
              value={value === "false" ? "false" : "true"}
              options={BOOLEAN_OPTIONS}
              onChange={setValue}
            />
          </>
        ) : (
          <label className="block">
            Value
            <input
              value={value}
              onChange={(event) => setValue(event.target.value)}
              inputMode={kind === "number" ? "decimal" : "text"}
              maxLength={4096}
              spellCheck={false}
              autoComplete="off"
              className={FIELD}
            />
          </label>
        )}
      </div>
      <FormError message={error} />
    </Dialog>
  );
}

export function ImportDialog({
  profileName,
  onImport,
  onClose,
}: {
  profileName: string;
  onImport: (text: string, replace: boolean) => Promise<string | null>;
  onClose: () => void;
}) {
  const [text, setText] = useState("");
  const [mode, setMode] = useState<"merge" | "replace">("merge");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const readFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (file.size > MAX_IMPORT_BYTES) {
      setError("That file is too large. The limit is 1 MB.");
      return;
    }
    try {
      setText(await file.text());
      setError(null);
    } catch {
      setError("That file couldn't be read.");
    }
  };

  const submit = async () => {
    setBusy(true);
    const failure = await onImport(text, mode === "replace");
    setBusy(false);
    if (failure) setError(failure);
    else onClose();
  };

  return (
    <Dialog
      title={`Import into ${profileName}`}
      onClose={onClose}
      actions={
        <>
          <Button variant="inset" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" disabled={busy || text.trim() === ""} onClick={() => void submit()}>
            Import flags
          </Button>
        </>
      }
    >
      <p className="text-[13px]">Choose a JSON file or paste a ClientAppSettings configuration.</p>
      <input
        type="file"
        accept=".json,application/json"
        aria-label="Choose a JSON file"
        onChange={(event) => void readFile(event)}
        className="mt-3 block w-full text-[13px] text-muted file:mr-3 file:h-8 file:rounded-md file:border-0 file:bg-hover file:px-3 file:text-text"
      />
      <textarea
        value={text}
        onChange={(event) => setText(event.target.value)}
        aria-label="Flag configuration JSON"
        placeholder={'{\n  "FIntExampleFlag": 1\n}'}
        spellCheck={false}
        rows={7}
        className="selectable mt-3 w-full resize-none rounded-lg bg-hover p-3 font-mono text-xs text-text placeholder:text-faint"
      />
      <div className="mt-3">
        <Segmented
          label="Import mode"
          value={mode}
          options={[
            { value: "merge", label: "Merge with current" },
            { value: "replace", label: "Replace all" },
          ]}
          onChange={setMode}
        />
      </div>
      <p className="mt-2 text-xs text-faint">A backup is saved first.</p>
      <FormError message={error} />
    </Dialog>
  );
}

