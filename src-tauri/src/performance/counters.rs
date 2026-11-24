//! Windows Performance Counters. The system reports these figures itself, so
//! BloxClient never opens, reads or attaches to the Roblox process.

use std::collections::HashMap;
use std::ptr::{null, null_mut};

use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER,
    PDH_HQUERY, PDH_MORE_DATA,
};

const ROBLOX_MEMORY: &str = r"\Process(RobloxPlayerBeta*)\Working Set - Private";
const GPU_ENGINES: &str = r"\GPU Engine(*)\Utilization Percentage";
const NET_DOWN: &str = r"\Network Interface(*)\Bytes Received/sec";
const NET_UP: &str = r"\Network Interface(*)\Bytes Sent/sec";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Reads one wildcard counter as (instance name, value) pairs.
///
/// # Safety
/// `counter` must belong to a query that is still open.
unsafe fn read_items(counter: PDH_HCOUNTER) -> Option<Vec<(String, f64)>> {
    let (mut size, mut count) = (0u32, 0u32);
    // SAFETY: a null buffer with size 0 only asks PDH for the required size.
    let status = unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, null_mut()) };
    if status != PDH_MORE_DATA || size == 0 {
        return None;
    }

    // u64 storage gives the 8-byte alignment the item structs need.
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    // SAFETY: `buffer` holds at least `size` bytes and outlives this call.
    let status = unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) };
    if status != 0 {
        return None;
    }

    // SAFETY: PDH wrote `count` items at the start of `buffer`.
    let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let state = item.FmtValue.CStatus;
        if state != PDH_CSTATUS_VALID_DATA && state != PDH_CSTATUS_NEW_DATA {
            continue;
        }
        // SAFETY: names are null-terminated strings inside `buffer`, and the
        // double member is the one requested by PDH_FMT_DOUBLE.
        let (name, value) = unsafe {
            let mut length = 0;
            while *item.szName.add(length) != 0 {
                length += 1;
            }
            let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, length));
            (name, item.FmtValue.Anonymous.doubleValue)
        };
        if value.is_finite() {
            values.push((name, value));
        }
    }
    Some(values)
}

/// The busiest GPU engine, the same way Task Manager reports overall GPU use.
/// Engines are shared between processes, so usage is summed per engine first.
pub fn busiest_engine(items: &[(String, f64)]) -> Option<f64> {
    let mut engines: HashMap<&str, f64> = HashMap::new();
    for (name, value) in items {
        let start = name.find("luid_")?;
        *engines.entry(&name[start..]).or_default() += value.max(0.0);
    }
    engines.into_values().reduce(f64::max).map(|busy| busy.min(100.0))
}

fn total(items: Option<Vec<(String, f64)>>) -> Option<f64> {
    let items = items?;
    (!items.is_empty()).then(|| items.iter().map(|(_, value)| value.max(0.0)).sum())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CounterValues {
    pub gpu_percent: Option<f64>,
    pub roblox_memory: Option<f64>,
    pub net_down: Option<f64>,
    pub net_up: Option<f64>,
}

pub struct Counters {
    query: PDH_HQUERY,
    gpu: Option<PDH_HCOUNTER>,
    roblox: Option<PDH_HCOUNTER>,
    net_down: Option<PDH_HCOUNTER>,
    net_up: Option<PDH_HCOUNTER>,
}

// SAFETY: PDH handles aren't tied to a thread, and every use goes through the
// mutex in `PerformanceMonitor`.
unsafe impl Send for Counters {}

impl Counters {
    pub fn open() -> Option<Self> {
        let mut query: PDH_HQUERY = null_mut();
        // SAFETY: a null data source means live data; `query` receives the handle.
        if unsafe { PdhOpenQueryW(null(), 0, &mut query) } != 0 {
            return None;
        }
        let add = |path: &str| -> Option<PDH_HCOUNTER> {
            let path = wide(path);
            let mut counter: PDH_HCOUNTER = null_mut();
            // SAFETY: `query` is open and `path` is null-terminated.
            let status = unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) };
            if status != 0 {
                log::warn!("performance counter unavailable: {status:#x}");
            }
            (status == 0).then_some(counter)
        };
        let counters = Self {
            gpu: add(GPU_ENGINES),
            roblox: add(ROBLOX_MEMORY),
            net_down: add(NET_DOWN),
            net_up: add(NET_UP),
            query,
        };
        // Rate counters need a first reading to compare the next one against.
        // SAFETY: `query` is open.
        unsafe { PdhCollectQueryData(query) };
        Some(counters)
    }

    pub fn read(&mut self) -> CounterValues {
        // SAFETY: `query` stays open until drop.
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            return CounterValues::default();
        }
        // SAFETY: each counter belongs to the open query.
        let read = |counter: Option<PDH_HCOUNTER>| counter.and_then(|handle| unsafe { read_items(handle) });
        CounterValues {
            gpu_percent: read(self.gpu).and_then(|items| busiest_engine(&items)),
            roblox_memory: total(read(self.roblox)),
            net_down: total(read(self.net_down)),
            net_up: total(read(self.net_up)),
        }
    }
}

impl Drop for Counters {
    fn drop(&mut self) {
        // SAFETY: the query is open and is never used after drop.
        unsafe { PdhCloseQuery(self.query) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, value: f64) -> (String, f64) {
        (name.to_string(), value)
    }

    #[test]
    fn shared_engines_are_summed_and_the_busiest_wins() {
        let items = [
            item("pid_1_luid_0x1_phys_0_eng_0_engtype_3D", 30.0),
            item("pid_2_luid_0x1_phys_0_eng_0_engtype_3D", 25.0),
            item("pid_1_luid_0x1_phys_0_eng_1_engtype_Copy", 40.0),
        ];
        assert_eq!(busiest_engine(&items), Some(55.0));
    }

    #[test]
    fn gpu_use_is_capped_and_needs_valid_names() {
        assert_eq!(busiest_engine(&[item("pid_1_luid_0x1_eng_0", 180.0)]), Some(100.0));
        assert_eq!(busiest_engine(&[item("unexpected", 10.0)]), None);
        assert_eq!(busiest_engine(&[]), None);
    }

    #[test]
    fn totals_distinguish_none_from_zero() {
        assert_eq!(total(None), None);
        assert_eq!(total(Some(vec![])), None);
        assert_eq!(total(Some(vec![item("a", 0.0)])), Some(0.0));
        assert_eq!(total(Some(vec![item("a", 2.0), item("b", 3.0)])), Some(5.0));
    }

    #[test]
    fn wildcard_process_counters_read_on_this_pc() {
        let mut query: PDH_HQUERY = null_mut();
        let path = wide(r"\Process(svchost*)\Working Set - Private");
        let mut counter: PDH_HCOUNTER = null_mut();
        // SAFETY: handles are local, valid for this test and closed below.
        let items = unsafe {
            assert_eq!(PdhOpenQueryW(null(), 0, &mut query), 0);
            assert_eq!(PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter), 0);
            assert_eq!(PdhCollectQueryData(query), 0);
            let items = read_items(counter);
            PdhCloseQuery(query);
            items
        };
        let items = items.expect("Windows always runs svchost");
        assert!(items.iter().any(|(_, value)| *value > 0.0));
    }

    #[test]
    fn counters_open_and_read_without_a_roblox_process() {
        let mut counters = Counters::open().expect("PDH should open");
        std::thread::sleep(std::time::Duration::from_millis(150));
        let values = counters.read();
        assert!(values.gpu_percent.is_none_or(|busy| (0.0..=100.0).contains(&busy)));
        assert!(values.net_down.is_none_or(|rate| rate >= 0.0));
    }
}
