use kube::api::ObjectMeta;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PodMetrics {
    pub metadata: ObjectMeta,
    pub containers: Vec<ContainerMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerMetrics {
    pub name: String,
    pub usage: ResourceUsage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: String,
    pub memory: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Statistics {
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub p95: u64,
    pub count: usize,
    pub sum: u64,
}

/// Which resource dimension the UI is currently displaying. The raw samples for
/// both dimensions are always kept in memory, so switching is a cheap re-render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Memory,
    Cpu,
}

impl Metric {
    /// Title shown above the table for this metric.
    pub fn title(&self) -> &'static str {
        match self {
            Metric::Memory => "Memory usage",
            Metric::Cpu => "CPU usage",
        }
    }

    /// Short label used in hints/toggles.
    pub fn label(&self) -> &'static str {
        match self {
            Metric::Memory => "memory",
            Metric::Cpu => "cpu",
        }
    }

    /// Format a raw value (bytes for memory, nanocores for cpu) for display.
    pub fn format(&self, value: u64) -> String {
        match self {
            Metric::Memory => memory_bytes_to_human(value),
            Metric::Cpu => cpu_nanos_to_human(value),
        }
    }

    /// The other metric — used to flip between modes.
    pub fn toggled(&self) -> Metric {
        match self {
            Metric::Memory => Metric::Cpu,
            Metric::Cpu => Metric::Memory,
        }
    }
}

pub fn parse_memory_bytes(s: &str) -> u64 {
    if let Some(n) = s.strip_suffix("Ki") {
        return n.parse::<u64>().unwrap_or(0) * 1024;
    }
    if let Some(n) = s.strip_suffix("Mi") {
        return n.parse::<u64>().unwrap_or(0) * 1024 * 1024;
    }
    if let Some(n) = s.strip_suffix("Gi") {
        return n.parse::<u64>().unwrap_or(0) * 1024 * 1024 * 1024;
    }

    s.parse::<u64>().unwrap_or(0)
}

/// Parse a Kubernetes CPU quantity into nanocores.
///
/// metrics-server usually reports integer nanocores (e.g. `"123456789n"`), but
/// the quantity format also allows micro (`u`), milli (`m`) and whole/fractional
/// cores, so all of those are handled.
pub fn parse_cpu_nanos(s: &str) -> u64 {
    if let Some(n) = s.strip_suffix('n') {
        return n.parse::<u64>().unwrap_or(0);
    }
    if let Some(n) = s.strip_suffix('u') {
        return n.parse::<u64>().unwrap_or(0) * 1_000;
    }
    if let Some(n) = s.strip_suffix('m') {
        return n.parse::<u64>().unwrap_or(0) * 1_000_000;
    }
    // Bare value is expressed in (possibly fractional) cores.
    s.parse::<f64>()
        .map(|cores| (cores * 1_000_000_000.0) as u64)
        .unwrap_or(0)
}

/// Render nanocores as millicores below one core, cores above it.
pub fn cpu_nanos_to_human(nanos: u64) -> String {
    if nanos >= 1_000_000_000 {
        format!("{:.2} cores", nanos as f64 / 1_000_000_000.0)
    } else {
        format!("{}m", (nanos as f64 / 1_000_000.0).round() as u64)
    }
}

pub fn memory_bytes_to_human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 * 1024 {
        return format!(
            "{:.2} TiB",
            bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
        );
    }
    if bytes >= 1024 * 1024 * 1024 {
        return format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0));
    }
    if bytes >= 1024 * 1024 {
        return format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0));
    }
    if bytes >= 1024 {
        format!("{:.2} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn calculate_stats(values: Vec<u64>) -> (u64, u64, f64, u64, usize, u64) {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let mean = if !values.is_empty() {
        values.iter().sum::<u64>() as f64 / values.len() as f64
    } else {
        0.0
    };
    let p95 = if !sorted.is_empty() {
        sorted[(sorted.len() as f64 * 0.95).ceil() as usize - 1]
    } else {
        0
    };

    (min, max, mean, p95, values.len(), values.iter().sum())
}
