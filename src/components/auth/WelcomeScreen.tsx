import { Logo } from "@/components/shell/Logo";
import { Button } from "@/components/ui/controls";
import { useAuth } from "@/state/auth";

/** First-launch screen. Signing in is optional. */
export function WelcomeScreen() {
  const { status, error, connect, cancel, dismissWelcome } = useAuth();

  // Only offered once a Roblox OAuth app has been set up.
  if (!status || !status.clientConfigured || status.welcomeSeen || status.state === "connected") return null;
  const signingIn = status.state === "signingIn";

  return (
    <div className="fade-in fixed inset-x-0 top-10 bottom-0 z-40 grid place-items-center bg-bg px-8">
      <div className="w-full max-w-md">
        <Logo size={56} />
        <h1 className="mt-6 font-display text-5xl leading-none font-bold tracking-tight">Welcome to BloxClient</h1>
        <p className="mt-4 text-lg text-muted">
          Connect your Roblox account through Roblox's own sign-in page, or start browsing without one.
        </p>
        {error && (
          <p role="alert" className="mt-4 text-[13px] text-danger">
            {error}
          </p>
        )}
        {signingIn ? (
          <div className="mt-7">
            <p className="mb-3 font-medium">Finish signing in in your browser.</p>
            <Button onClick={() => void cancel()}>Cancel sign-in</Button>
          </div>
        ) : (
          <div className="mt-7 flex flex-wrap gap-2">
            <Button variant="primary" onClick={() => void connect()}>
              Connect Roblox account
            </Button>
            <Button onClick={() => void dismissWelcome()}>Continue without an account</Button>
          </div>
        )}
        <p className="mt-6 text-xs text-faint">
          BloxClient is an independent app, not made by Roblox. It never asks for your password or cookie.
        </p>
      </div>
    </div>
  );
}
