import { Button } from "@/components/ui/controls";

export function LoadError({ title, message, onRetry }: { title: string; message: string; onRetry: () => void }) {
  return (
    <div role="alert" className="px-8 py-12">
      <h2 className="font-display text-2xl font-semibold">{title}</h2>
      <p className="mt-1 mb-4 max-w-prose text-muted">{message}</p>
      <Button onClick={onRetry}>Try again</Button>
    </div>
  );
}
