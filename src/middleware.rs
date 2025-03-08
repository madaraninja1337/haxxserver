use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::time::{Instant, Duration};

static RATE_LIMIT: Lazy<DashMap<SocketAddr, (u32, Instant)>> = Lazy::new(|| DashMap::new());
const MAX_REQUESTS: u32 = 10;
const WINDOW_SECS: u64 = 10;

pub fn check_rate_limit(remote: SocketAddr) -> bool {
    let now = Instant::now();
    let mut entry = RATE_LIMIT.entry(remote).or_insert((0, now));
    let elapsed = now.duration_since(entry.value().1);
    if elapsed > Duration::from_secs(WINDOW_SECS) {
        *entry.value_mut() = (1, now);
        true
    } else {
        entry.value_mut().0 += 1;
        entry.value().0 <= MAX_REQUESTS
    }
}

pub fn metrics() -> String {
    let mut out = String::new();
    for item in RATE_LIMIT.iter() {
        out.push_str(&format!("{}: {} requests\n", item.key(), item.value().0));
    }
    out
}
