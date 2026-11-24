import { useState } from "react";

/** A game image with a plain fallback for missing or expired images. */
export function Thumbnail({
  url,
  name,
  className = "",
  eager = false,
}: {
  url: string | null;
  name: string;
  className?: string;
  eager?: boolean;
}) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const usable = url !== null && url !== failedUrl;

  if (!usable) {
    return (
      <div aria-hidden="true" className={`grid place-items-center bg-raised font-display text-3xl text-faint ${className}`}>
        {name.trim().charAt(0).toUpperCase()}
      </div>
    );
  }
  return (
    <img
      src={url}
      alt=""
      draggable={false}
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      onError={() => setFailedUrl(url)}
      className={`bg-raised object-cover ${className}`}
    />
  );
}
