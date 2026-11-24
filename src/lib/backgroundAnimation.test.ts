import { describe, expect, it, vi } from "vitest";
import { watchBackgroundAnimation } from "./backgroundAnimation";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, reject, resolve };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("watchBackgroundAnimation", () => {
  it("reads only after the listener is active", async () => {
    const subscribed = deferred<() => void>();
    const read = vi.fn(() => Promise.resolve(true));
    const onChange = vi.fn();
    watchBackgroundAnimation({ read, subscribe: () => subscribed.promise }, onChange);

    await settle();
    expect(read).not.toHaveBeenCalled();

    subscribed.resolve(() => undefined);
    await settle();
    expect(read).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("does not let a slow read replace a newer event", async () => {
    const value = deferred<boolean>();
    let emit: (allowed: boolean) => void = () => undefined;
    const onChange = vi.fn();
    watchBackgroundAnimation(
      {
        read: () => value.promise,
        subscribe: (handler) => {
          emit = handler;
          return Promise.resolve(() => undefined);
        },
      },
      onChange,
    );

    await settle();
    emit(true);
    value.resolve(false);
    await settle();
    expect(onChange.mock.calls).toEqual([[true]]);
  });

  it("still reads the value if listening fails", async () => {
    const onChange = vi.fn();
    watchBackgroundAnimation(
      {
        read: () => Promise.resolve(true),
        subscribe: () => Promise.reject(new Error("no listener")),
      },
      onChange,
    );

    await settle();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("removes a listener that finishes starting after cleanup", async () => {
    const subscribed = deferred<() => void>();
    const unsubscribe = vi.fn();
    const read = vi.fn(() => Promise.resolve(true));
    const stop = watchBackgroundAnimation({ read, subscribe: () => subscribed.promise }, vi.fn());

    stop();
    subscribed.resolve(unsubscribe);
    await settle();
    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(read).not.toHaveBeenCalled();
  });
});
