import { useState } from "react";
import { Button } from "./controls";
import { Dialog } from "./Dialog";

export function NameDialog({
  title,
  label,
  initial,
  action,
  onSubmit,
  onClose,
}: {
  title: string;
  label: string;
  initial: string;
  action: string;
  onSubmit: (name: string) => Promise<string | null>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    const failure = await onSubmit(name);
    if (failure) setError(failure);
    else onClose();
  };

  return (
    <Dialog
      title={title}
      onClose={onClose}
      actions={
        <>
          <Button variant="inset" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" disabled={name.trim() === ""} onClick={() => void submit()}>
            {action}
          </Button>
        </>
      }
    >
      <label className="mt-2 block text-[13px]">
        {label}
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && name.trim() !== "") void submit();
          }}
          maxLength={40}
          spellCheck={false}
          autoComplete="off"
          className="selectable mt-1 h-9 w-full rounded-lg bg-hover px-3 text-[13px] text-text placeholder:text-faint"
        />
      </label>
      {error && (
        <p role="alert" className="mt-3 text-[13px] text-danger">
          {error}
        </p>
      )}
    </Dialog>
  );
}

export function ConfirmDialog({
  title,
  body,
  action,
  onConfirm,
  onClose,
}: {
  title: string;
  body: string;
  action: string;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog
      title={title}
      onClose={onClose}
      actions={
        <>
          <Button variant="inset" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              onClose();
              onConfirm();
            }}
          >
            {action}
          </Button>
        </>
      }
    >
      {body}
    </Dialog>
  );
}
