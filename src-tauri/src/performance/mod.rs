//! Live system stats, sampled only while the Performance page asks for them.
//! There is no background thread, and nothing touches the Roblox process.

mod counters;

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::{redirect, Client};
use serde::Serialize;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use crate::error::{AppError, AppResult};
use crate::roblox::{is_player_running, RobloxError};
use counters::{CounterValues, Counters};

const MAX_POINTS: usize = 300;
const MIN_SAMPLE_GAP: Duration = Duration::from_millis(900);
/// A longer pause means the page was closed, so old points would mislead.
const MAX_HISTORY_GAP: Duration = Duration::from_secs(15);
const PROBE_TIMEOUT: Duration = Duration::from_secs(6);
const PROBE_HOSTS: [&str; 4] = ["www.roblox.com", "apis.roblox.com", "setup.rbxcdn.com", "tr.rbxcdn.com"];

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct PerfPoint {
    /// Unix milliseconds.
    pub at: f64,
    pub cpu_percent: f64,
    pub memory_used: f64,
    pub memory_total: f64,
    pub gpu_percent: Option<f64>,
    pub roblox_running: bool,
    pub roblox_memory: Option<f64>,
    pub net_down: Option<f64>,
    pub net_up: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct NetworkProbe {
    pub host: String,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
}

struct Sampler {
    system: System,
    counters: Option<Counters>,
    history: VecDeque<PerfPoint>,
    last: Option<Instant>,
}

/// Adds a point, starting over after a gap in sampling.
fn push_point(history: &mut VecDeque<PerfPoint>, point: PerfPoint, gap: Option<Duration>) {
    if gap.is_none_or(|elapsed| elapsed > MAX_HISTORY_GAP) {
        history.clear();
    }
    history.push_back(point);
    while history.len() > MAX_POINTS {
        history.pop_front();
    }
}

fn build_point(at: f64, cpu: f64, used: f64, total: f64, running: bool, values: CounterValues) -> PerfPoint {
    PerfPoint {
        at,
        cpu_percent: cpu.clamp(0.0, 100.0),
        memory_used: used,
        memory_total: total,
        gpu_percent: values.gpu_percent,
        roblox_running: running,
        // A leftover counter value isn't meaningful once Roblox has closed.
        roblox_memory: values.roblox_memory.filter(|_| running),
        net_down: values.net_down,
        net_up: values.net_up,
    }
}

pub struct PerformanceMonitor {
    sampler: Mutex<Sampler>,
    client: Client,
}

impl PerformanceMonitor {
    pub fn new() -> AppResult<Self> {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let client = Client::builder()
            .user_agent(concat!("BloxClient/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .redirect(redirect::Policy::none())
            .timeout(PROBE_TIMEOUT)
            // A fresh connection each time, so the check measures the network.
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|error| AppError::Roblox(RobloxError::Network(error.to_string())))?;
        Ok(Self {
            sampler: Mutex::new(Sampler {
                system,
                // Opened on first use, so an unused page costs nothing.
                counters: None,
                history: VecDeque::with_capacity(MAX_POINTS),
                last: None,
            }),
            client,
        })
    }

    pub fn history(&self) -> Vec<PerfPoint> {
        let sampler = self.sampler.lock().unwrap_or_else(PoisonError::into_inner);
        let current = sampler.last.is_some_and(|last| last.elapsed() <= MAX_HISTORY_GAP);
        if current {
            sampler.history.iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    pub fn sample(&self) -> Option<PerfPoint> {
        let mut sampler = self.sampler.lock().unwrap_or_else(PoisonError::into_inner);
        let gap = sampler.last.map(|last| last.elapsed());
        if gap.is_some_and(|elapsed| elapsed < MIN_SAMPLE_GAP) {
            return sampler.history.back().copied();
        }

        if sampler.counters.is_none() {
            sampler.counters = Counters::open();
        }
        sampler.system.refresh_cpu_usage();
        sampler.system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        let values = sampler.counters.as_mut().map_or_else(CounterValues::default, Counters::read);
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |elapsed| elapsed.as_millis() as f64);
        let point = build_point(
            at,
            f64::from(sampler.system.global_cpu_usage()),
            sampler.system.used_memory() as f64,
            sampler.system.total_memory() as f64,
            is_player_running(),
            values,
        );
        push_point(&mut sampler.history, point, gap);
        sampler.last = Some(Instant::now());
        Some(point)
    }

    async fn probe(&self, host: &str) -> NetworkProbe {
        let started = Instant::now();
        // Any HTTP reply proves the host is reachable, whatever its status.
        match self.client.head(format!("https://{host}/")).send().await {
            Ok(_) => NetworkProbe {
                host: host.to_string(),
                reachable: true,
                latency_ms: Some(started.elapsed().as_secs_f64() * 1000.0),
            },
            Err(error) => {
                log::warn!("network check failed for {host}: {error}");
                NetworkProbe { host: host.to_string(), reachable: false, latency_ms: None }
            }
        }
    }

    pub async fn check_network(&self) -> Vec<NetworkProbe> {
        let [first, second, third, fourth] = PROBE_HOSTS;
        let (a, b, c, d) = tokio::join!(
            self.probe(first),
            self.probe(second),
            self.probe(third),
            self.probe(fourth)
        );
        vec![a, b, c, d]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(at: f64) -> PerfPoint {
        build_point(at, 10.0, 1.0, 2.0, false, CounterValues::default())
    }

    #[test]
    fn history_is_capped() {
        let mut history = VecDeque::new();
        for index in 0..(MAX_POINTS + 5) {
            push_point(&mut history, point(index as f64), Some(Duration::from_secs(2)));
        }
        assert_eq!(history.len(), MAX_POINTS);
        assert_eq!(history.front().unwrap().at, 5.0);
    }

    #[test]
    fn a_gap_starts_a_new_history() {
        let mut history = VecDeque::new();
        push_point(&mut history, point(1.0), None);
        push_point(&mut history, point(2.0), Some(Duration::from_secs(2)));
        assert_eq!(history.len(), 2);
        push_point(&mut history, point(3.0), Some(MAX_HISTORY_GAP + Duration::from_secs(1)));
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn roblox_memory_is_only_reported_while_it_runs() {
        let values = CounterValues { roblox_memory: Some(500.0), ..CounterValues::default() };
        assert_eq!(build_point(0.0, 1.0, 1.0, 2.0, false, values).roblox_memory, None);
        assert_eq!(build_point(0.0, 1.0, 1.0, 2.0, true, values).roblox_memory, Some(500.0));
    }

    #[test]
    fn cpu_is_clamped() {
        assert_eq!(build_point(0.0, 140.0, 1.0, 2.0, false, CounterValues::default()).cpu_percent, 100.0);
        assert_eq!(build_point(0.0, -1.0, 1.0, 2.0, false, CounterValues::default()).cpu_percent, 0.0);
    }

    #[test]
    fn samples_real_system_memory() {
        let monitor = PerformanceMonitor::new().unwrap();
        let sample = monitor.sample().unwrap();
        assert!(sample.memory_total > 0.0);
        assert!(sample.memory_used <= sample.memory_total);
        assert_eq!(monitor.history().len(), 1);
    }
}
