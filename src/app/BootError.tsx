export function BootError({ message }: { message: string }) {
  return (
    <div role="alert" className="flex h-full flex-col justify-center gap-2 bg-bg px-12">
      <h1 className="font-display text-3xl font-bold">BloxClient couldn't start</h1>
      <p className="max-w-prose text-muted">{message}</p>
    </div>
  );
}
