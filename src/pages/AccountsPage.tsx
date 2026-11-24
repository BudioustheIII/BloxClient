import { Check, Copy, ExternalLink } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Thumbnail } from "@/components/experience/Thumbnail";
import { Button } from "@/components/ui/controls";
import type { Account } from "@/lib/bindings/Account";
import { useAuth } from "@/state/auth";
import { LocalProfiles } from "./accounts/LocalProfiles";

function CopyValue({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    });
  };
  return (
    <span className="mt-1.5 flex items-center gap-2">
      <code className="selectable min-w-0 flex-1 truncate rounded-md bg-raised px-3 py-2 text-[13px]">{value}</code>
      <Button aria-label={`Copy ${value}`} onClick={copy}>
        {copied ? <Check size={15} aria-hidden="true" /> : <Copy size={15} aria-hidden="true" />}
        {copied ? "Copied" : "Copy"}
      </Button>
    </span>
  );
}

function SetupPanel({ redirectUri }: { redirectUri: string }) {
  const { saveClientId, openDashboard } = useAuth();
  const [clientId, setClientId] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    if (await saveClientId(clientId)) setClientId("");
    setSaving(false);
  };

  return (
    <section className="mt-8">
      <h2 className="font-display text-xl font-semibold">Set up Roblox sign-in</h2>
      <p className="mt-1 text-muted">
        Roblox requires each app to be registered once. Registering needs an ID-verified Roblox account.
      </p>
      <ol className="mt-5 space-y-5">
        <li>
          <p className="font-medium">1. Create an OAuth app in Creator Dashboard</p>
          <Button className="mt-1.5" onClick={openDashboard}>
            <ExternalLink size={15} aria-hidden="true" />
            Open Creator Dashboard
          </Button>
        </li>
        <li>
          <p className="font-medium">2. Add this redirect URL to the app</p>
          <CopyValue value={redirectUri} />
        </li>
        <li>
          <p className="font-medium">3. Turn on the openid and profile permissions</p>
          <p className="text-[13px] text-muted">BloxClient doesn't need any others.</p>
        </li>
        <li>
          <p className="font-medium">4. Paste the app's client ID</p>
          <p className="text-[13px] text-muted">Only the client ID. Don't enter the app secret here.</p>
          <form onSubmit={(event) => void submit(event)} className="mt-1.5 flex gap-2">
            <input
              value={clientId}
              onChange={(event) => setClientId(event.target.value)}
              aria-label="OAuth client ID"
              placeholder="Client ID"
              spellCheck={false}
              autoComplete="off"
              className="selectable h-9 min-w-0 flex-1 rounded-lg bg-raised px-3 text-[13px] text-text placeholder:text-faint"
            />
            <Button type="submit" disabled={saving || clientId.trim() === ""}>
              Save client ID
            </Button>
          </form>
        </li>
      </ol>
    </section>
  );
}

function ConnectedAccount({ account }: { account: Account }) {
  const { signOut } = useAuth();
  const [confirming, setConfirming] = useState(false);

  return (
    <section className="mt-8">
      <div className="flex items-center gap-4">
        <Thumbnail url={account.avatarUrl} name={account.displayName} className="size-20 rounded-full" />
        <div className="min-w-0">
          <p className="truncate font-display text-2xl font-semibold">{account.displayName}</p>
          <p className="truncate text-muted">@{account.username}</p>
        </div>
      </div>
      <div className="mt-6 flex gap-2">
        {confirming ? (
          <>
            <Button onClick={() => setConfirming(false)}>Stay connected</Button>
            <Button variant="danger" onClick={() => void signOut()}>
              Log out
            </Button>
          </>
        ) : (
          <Button onClick={() => setConfirming(true)}>Log out</Button>
        )}
      </div>
      <p className="mt-6 max-w-prose text-[13px] text-muted">
        The Roblox Player keeps its own login, so games run on whichever account is signed in there.
      </p>
    </section>
  );
}

export default function AccountsPage() {
  const { status, error, connect, cancel } = useAuth();

  return (
    <div className="max-w-2xl px-10 py-9">
      <h1 className="font-display text-4xl font-bold tracking-tight">Accounts</h1>

      {error && (
        <p role="alert" className="mt-4 text-danger">
          {error}
        </p>
      )}

      {status?.state === "connected" && status.account && <ConnectedAccount account={status.account} />}

      {status?.state === "signingIn" && (
        <section className="mt-8">
          <h2 className="font-display text-xl font-semibold">Finish signing in in your browser</h2>
          <p className="mt-1 mb-4 text-muted">When your browser asks to open BloxClient, allow it.</p>
          <Button onClick={() => void cancel()}>Cancel sign-in</Button>
        </section>
      )}

      {status?.state === "signedOut" && status.clientConfigured && (
        <section className="mt-8">
          <h2 className="font-display text-xl font-semibold">Connect your Roblox account</h2>
          <p className="mt-1 mb-4 max-w-prose text-muted">
            You'll sign in on Roblox's official page in your browser. BloxClient never sees your password.
          </p>
          <Button variant="primary" onClick={() => void connect()}>
            Connect Roblox account
          </Button>
        </section>
      )}

      {status?.state === "signedOut" && !status.clientConfigured && <SetupPanel redirectUri={status.redirectUri} />}

      <LocalProfiles />

      <section className="mt-10">
        <h2 className="font-display text-lg font-semibold">How your account is protected</h2>
        <p className="mt-1 max-w-prose text-[13px] text-muted">
          BloxClient stores no password, cookie or sign-in token. It uses Roblox's token once to read your profile, then
          discards it. Your name, avatar link, saved games, people and play history are encrypted for your Windows account
          on this PC.
        </p>
      </section>
    </div>
  );
}
