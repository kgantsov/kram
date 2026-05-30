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
