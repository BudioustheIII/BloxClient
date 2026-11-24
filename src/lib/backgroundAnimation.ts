export interface BackgroundAnimationSource {
  /** Starts listening for changes and resolves once the listener is active. */
  subscribe(handler: (allowed: boolean) => void): Promise<() => void>;
  /** Reads the current value. */
  read(): Promise<boolean>;
}

/**
 * Follows whether the background may animate. It reads the value only after
 * the listener is active, so a change during startup can't be missed, and a
 * slow read can never replace a newer event.
 */
export function watchBackgroundAnimation(
  source: BackgroundAnimationSource,
  onChange: (allowed: boolean) => void,
): () => void {
  let active = true;
  let eventSeen = false;
  let stop: (() => void) | null = null;

  const readCurrent = () => {
    source.read().then(
      (allowed) => {
        if (active && !eventSeen) onChange(allowed);
      },
      () => undefined,
    );
  };

  source
    .subscribe((allowed) => {
      if (!active) return;
      eventSeen = true;
      onChange(allowed);
    })
    .then((unsubscribe) => {
      if (!active) {
        unsubscribe();
        return;
      }
      stop = unsubscribe;
      readCurrent();
    }, readCurrent);

  return () => {
    active = false;
    stop?.();
  };
}
