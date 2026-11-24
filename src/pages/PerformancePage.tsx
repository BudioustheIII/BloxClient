import { Wifi } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { LineChart } from "@/components/charts/LineChart";
import { Button } from "@/components/ui/controls";
import type { NetworkProbe } from "@/lib/bindings/NetworkProbe";
import type { PerfPoint } from "@/lib/bindings/PerfPoint";
import { formatDuration } from "@/lib/experience";
import { formatBytes } from "@/lib/format";
import { api, failureMessage } from "@/lib/ipc";
import { formatRate, type ChartPoint } from "@/lib/performance";
import { usePlay } from "@/state/play";

const SAMPLE_MS = 2_000;
const MAX_POINTS = 300;
const TABLE_ROWS = 8;
const MIN_RATE_SCALE = 128 * 1024;
const timeFormat = new Intl.DateTimeFormat(undefined, { timeStyle: "medium" });

const percent = (value: number) => `${Math.round(value)}%`;

function useSamples() {
  const [points, setPoints] = useState<PerfPoint[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let busy = false;

    const sample = async () => {
      // A hidden page stops sampling, so an unseen graph costs nothing.
      if (busy || document.visibilityState !== "visible") return;
      busy = true;
      try {
        const next = await api.samplePerformance();
        if (active && next) {
          setError(null);
          setPoints((current) =>
            current[current.length - 1]?.at === next.at ? current : [...current, next].slice(-MAX_POINTS),
          );
        }
      } catch (reason) {
        if (active) setError(failureMessage(reason, "Live readings aren't available."));
      } finally {
        busy = false;
      }
    };

    void api.performanceHistory().then((history) => {
      if (active) setPoints((current) => (current.length === 0 ? history : current));
      void sample();
    });
    const timer = window.setInterval(() => void sample(), SAMPLE_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  return { points, error };
}

function series(points: readonly PerfPoint[], read: (point: PerfPoint) => number | null): ChartPoint[] {
  return points.map((point) => ({ at: point.at, value: read(point) }));
}

function highest(points: readonly ChartPoint[], floor: number): number {
  return Math.max(floor, ...points.map((point) => point.value ?? 0)) * 1.15;
}

function NetworkCheck() {
  const [results, setResults] = useState<NetworkProbe[] | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setRunning(true);
    setError(null);
    try {
      setResults(await api.checkNetwork());
    } catch (reason) {
      setError(failureMessage(reason, "The check didn't finish."));
    } finally {
      setRunning(false);
    }
  };

  return (
    <section className="mt-8">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="font-display text-xl font-semibold">Connection to Roblox</h2>
          <p className="text-[13px] text-muted">Times one secure connection to each Roblox service.</p>
        </div>
        <Button disabled={running} onClick={() => void run()}>
          <Wifi size={15} aria-hidden="true" />
          {running ? "Checking" : "Run check"}
        </Button>
      </div>
      {error && (
        <p role="alert" className="mt-2 text-[13px] text-danger">
          {error}
        </p>
      )}
      {results && (
        <ul className="mt-3">
          {results.map((probe) => (
            <li key={probe.host} className="flex items-center justify-between gap-4 py-1.5 text-[13px]">
              <span className="selectable truncate">{probe.host}</span>
              <span className={probe.reachable ? "text-muted tabular-nums" : "text-danger"}>
                {probe.latencyMs !== null ? `${Math.round(probe.latencyMs)} ms` : "Not reachable"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function SessionStats() {
  const { session, recent } = usePlay();
  const [now, setNow] = useState(() => Date.now());
  const runningSince = session?.runningSince ?? null;

  useEffect(() => {
    if (runningSince === null) return;
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, [runningSince]);

  const tracked = recent.reduce((total, game) => total + game.totalSeconds, 0);
  const launches = recent.reduce((total, game) => total + game.playCount, 0);
  const top = [...recent]
    .filter((game) => game.totalSeconds > 0)
    .sort((a, b) => b.totalSeconds - a.totalSeconds)
    .slice(0, 5);
  const stats = [
    {
      label: "Current session",
      value: session && runningSince !== null ? formatDuration(now / 1000 - runningSince) : "None",
    },
    { label: "Time tracked", value: tracked > 0 ? formatDuration(tracked) : "None yet" },
    { label: "Launches tracked", value: String(launches) },
  ];

  return (
    <section className="mt-8">
      <h2 className="font-display text-xl font-semibold">Sessions</h2>
      <p className="text-[13px] text-muted">
        Counts your {recent.length} most recent games. Time is only tracked for games started from BloxClient.
      </p>
      <dl className="mt-3 grid grid-cols-3 gap-4">
        {stats.map((stat) => (
          <div key={stat.label}>
            <dt className="text-[13px] text-muted">{stat.label}</dt>
            <dd className="font-display text-lg font-semibold">{stat.value}</dd>
          </div>
        ))}
      </dl>
      {top.length > 0 && (
        <ul className="mt-3">
          {top.map((game) => (
            <li key={game.universeId} className="flex justify-between gap-4 py-1 text-[13px]">
              <span className="truncate">{game.name}</span>
              <span className="shrink-0 text-muted">{formatDuration(game.totalSeconds)}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export default function PerformancePage() {
  const { points, error } = useSamples();
  const latest = points[points.length - 1];

  const charts = useMemo(() => {
    const down = series(points, (point) => point.netDown);
    const up = series(points, (point) => point.netUp);
    const roblox = series(points, (point) => point.robloxMemory);
    return {
      cpu: series(points, (point) => point.cpuPercent),
      gpu: series(points, (point) => point.gpuPercent),
      memory: series(points, (point) => point.memoryUsed),
      roblox,
      down,
      up,
      robloxMax: highest(roblox, 512 * 1024 * 1024),
      downMax: highest(down, MIN_RATE_SCALE),
      upMax: highest(up, MIN_RATE_SCALE),
    };
  }, [points]);

  const rows = points.slice(-TABLE_ROWS).reverse();

  return (
    <div className="max-w-5xl px-8 py-8">
      <h1 className="font-display text-4xl font-bold tracking-tight">Performance</h1>
      <p className="mt-2 max-w-prose text-[13px] text-muted">
        Figures come from Windows and update every 2 seconds while this page is open. BloxClient doesn't touch the Roblox
        process, so in-game FPS isn't available.
      </p>
      <p role="status" className="mt-3 text-[13px]">
        {latest?.robloxRunning ? "Roblox is running." : "Roblox isn't running. Open it to see its memory use here."}
      </p>
      {error && (
        <p role="alert" className="mt-2 text-[13px] text-danger">
          {error}
        </p>
      )}

      <div className="mt-5 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <LineChart title="CPU" points={charts.cpu} max={100} format={percent} />
        <LineChart title="GPU" points={charts.gpu} max={100} format={percent} />
        <LineChart title="System memory" points={charts.memory} max={latest?.memoryTotal ?? 1} format={formatBytes} />
        <LineChart title="Roblox memory" points={charts.roblox} max={charts.robloxMax} format={formatBytes} />
        <LineChart title="Download" points={charts.down} max={charts.downMax} format={formatRate} />
        <LineChart title="Upload" points={charts.up} max={charts.upMax} format={formatRate} />
      </div>

      <details className="mt-4 text-[13px]">
        <summary className="cursor-pointer text-muted hover:text-text">Show recent readings as a table</summary>
        <div className="mt-2 overflow-x-auto">
          <table className="w-full text-left tabular-nums">
            <thead className="text-muted">
              <tr>
                {["Time", "CPU", "GPU", "Memory", "Roblox", "Down", "Up"].map((heading) => (
                  <th key={heading} scope="col" className="py-1 pr-4 font-medium">
                    {heading}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((point) => (
                <tr key={point.at}>
                  <td className="py-1 pr-4">{timeFormat.format(new Date(point.at))}</td>
                  <td className="py-1 pr-4">{percent(point.cpuPercent)}</td>
                  <td className="py-1 pr-4">{point.gpuPercent === null ? "—" : percent(point.gpuPercent)}</td>
                  <td className="py-1 pr-4">{formatBytes(point.memoryUsed)}</td>
                  <td className="py-1 pr-4">{point.robloxMemory === null ? "—" : formatBytes(point.robloxMemory)}</td>
                  <td className="py-1 pr-4">{point.netDown === null ? "—" : formatRate(point.netDown)}</td>
                  <td className="py-1 pr-4">{point.netUp === null ? "—" : formatRate(point.netUp)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>

      <NetworkCheck />
      <SessionStats />
    </div>
  );
}
