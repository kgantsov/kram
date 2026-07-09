use comfy_table::Table;
use k8s_openapi::api::apps::v1::ReplicaSet;
use std::collections::HashMap;

use crate::command::{SortBy, SortOrder};
use crate::kubernetes::{get_metrics, get_pods, get_replicasets, resolve_pod_owner};
use crate::metrics::{
    Metric, PodMetrics, Statistics, calculate_stats, parse_cpu_nanos, parse_memory_bytes,
};
use kube::Client;

/// Raw per-workload samples for a single `<owner-kind>/<owner-name>/<container>`.
///
/// Both memory (bytes) and cpu (nanocores) values are kept so the UI can switch
/// metrics without re-querying the cluster. Each vector holds one entry per
/// observed pod-container.
pub struct ResourceSamples {
    pub resource: String,
    pub memory: Vec<u64>,
    pub cpu: Vec<u64>,
}

impl ResourceSamples {
    /// Raw values for the given metric.
    pub fn values(&self, metric: Metric) -> &[u64] {
        match metric {
            Metric::Memory => &self.memory,
            Metric::Cpu => &self.cpu,
        }
    }
}

/// One workload's statistics together with its resource label.
pub struct StatsRow {
    pub resource: String,
    pub stats: Statistics,
}

/// Fully computed table: per-workload rows (already sorted) and a summary row.
pub struct TableData {
    pub rows: Vec<StatsRow>,
    pub summary: Statistics,
}

/// Sort rows in place by table column index (0 = resource name, 1..=6 = stats).
///
/// All numeric columns are compared on their raw values (bytes / counts), never
/// on the human-readable strings, so e.g. `1.48 GiB` correctly sorts above
/// `512 MiB`.
pub fn sort_rows(rows: &mut [StatsRow], column: usize, desc: bool) {
    match column {
        0 => rows.sort_by(|a, b| a.resource.cmp(&b.resource)),
        1 => rows.sort_by_key(|r| r.stats.min),
        2 => rows.sort_by_key(|r| r.stats.max),
        3 => rows.sort_by(|a, b| a.stats.mean.total_cmp(&b.stats.mean)),
        4 => rows.sort_by_key(|r| r.stats.p95),
        5 => rows.sort_by_key(|r| r.stats.count),
        6 => rows.sort_by_key(|r| r.stats.sum),
        _ => return,
    }
    if desc {
        rows.reverse();
    }
}

/// Map a CLI `SortBy` variant to its table column index (matches `sort_rows`).
pub fn sort_by_to_column(sort_by: &SortBy) -> usize {
    match sort_by {
        SortBy::Min => 1,
        SortBy::Max => 2,
        SortBy::Mean => 3,
        SortBy::P95 => 4,
        SortBy::Count => 5,
        SortBy::Sum => 6,
    }
}

/// Fetch pods/metrics from the cluster and build the raw per-workload samples.
///
/// This is the single cluster round-trip: everything the UI needs (both memory
/// and cpu values for every workload) is captured here so that later metric
/// switches and re-sorts are pure in-memory operations.
pub async fn collect_raw(
    namespace: Option<String>,
    selector: Vec<String>,
) -> anyhow::Result<Vec<ResourceSamples>> {
    let client = Client::try_default().await?;
    let (pods, replicasets, metrics_list) = tokio::try_join!(
        get_pods(client.clone(), namespace.clone(), selector),
        get_replicasets(client.clone(), namespace.clone()),
        get_metrics(client.clone(), namespace),
    )?;

    let rs_map: HashMap<String, &ReplicaSet> = replicasets
        .iter()
        .filter_map(|rs| {
            let ns = rs.metadata.namespace.as_deref().unwrap_or("default");
            let name = rs.metadata.name.as_deref()?;
            Some((format!("{ns}/{name}"), rs))
        })
        .collect();

    let mut owner_map: HashMap<String, String> = HashMap::new();
    for pod in &pods {
        let owner = resolve_pod_owner(pod, &rs_map);
        owner_map.insert(pod.metadata.name.clone().unwrap_or_default(), owner);
    }

    let mut owner_samples: HashMap<String, ResourceSamples> = HashMap::new();
    for metric in &metrics_list {
        let pod_name = metric.metadata.name.clone().unwrap_or_default();
        let Some(pod_owner) = owner_map.get(&pod_name).cloned() else {
            continue;
        };

        let pod_metrics: PodMetrics = serde_json::from_value(serde_json::to_value(metric)?)?;
        for container in &pod_metrics.containers {
            let resource = format!("{pod_owner}/{}", container.name);
            let entry = owner_samples
                .entry(resource.clone())
                .or_insert_with(|| ResourceSamples {
                    resource,
                    memory: Vec::new(),
                    cpu: Vec::new(),
                });
            entry
                .memory
                .push(parse_memory_bytes(&container.usage.memory));
            entry.cpu.push(parse_cpu_nanos(&container.usage.cpu));
        }
    }

    Ok(owner_samples.into_values().collect())
}

/// Compute per-workload statistics for `metric` from raw samples and sort them.
///
/// `filter` restricts the result to workloads whose resource label contains it
/// (case-insensitive); an empty filter includes everything. The summary row is
/// computed over the filtered set so it always describes what is on screen.
pub fn build_table_data(
    samples: &[ResourceSamples],
    metric: Metric,
    sort_col: usize,
    sort_desc: bool,
    filter: &str,
) -> TableData {
    let filter = filter.to_lowercase();
    let mut all_values: Vec<u64> = Vec::new();
    let mut rows: Vec<StatsRow> = Vec::new();

    for sample in samples {
        if !filter.is_empty() && !sample.resource.to_lowercase().contains(&filter) {
            continue;
        }
        let values = sample.values(metric);
        if values.is_empty() {
            continue;
        }
        let (min, max, mean, p95, count, sum) = calculate_stats(values.to_vec());
        rows.push(StatsRow {
            resource: sample.resource.clone(),
            stats: Statistics {
                min,
                max,
                mean,
                p95,
                count,
                sum,
            },
        });
        all_values.extend_from_slice(values);
    }

    sort_rows(&mut rows, sort_col, sort_desc);

    let summary = if all_values.is_empty() {
        Statistics {
            min: 0,
            max: 0,
            mean: 0.0,
            p95: 0,
            count: 0,
            sum: 0,
        }
    } else {
        let (min, max, mean, p95, count, sum) = calculate_stats(all_values);
        Statistics {
            min,
            max,
            mean,
            p95,
            count,
            sum,
        }
    };

    TableData { rows, summary }
}

/// Print the memory statistics table to stdout using `comfy-table` (non-TUI mode).
pub async fn run(
    namespace: Option<String>,
    selector: Vec<String>,
    sort_order: SortOrder,
    sort_by: SortBy,
) -> anyhow::Result<()> {
    let samples = collect_raw(namespace, selector).await?;
    let data = build_table_data(
        &samples,
        Metric::Memory,
        sort_by_to_column(&sort_by),
        matches!(sort_order, SortOrder::Desc),
        "",
    );

    let mut table = Table::new();
    table.set_header(vec![
        "Resource", "min", "max", "mean", "p95", "count", "sum",
    ]);

    for StatsRow { resource, stats } in &data.rows {
        table.add_row(vec![
            resource.clone(),
            Metric::Memory.format(stats.min),
            Metric::Memory.format(stats.max),
            Metric::Memory.format(stats.mean as u64),
            Metric::Memory.format(stats.p95),
            stats.count.to_string(),
            Metric::Memory.format(stats.sum),
        ]);
    }

    let s = &data.summary;
    table.add_row(vec![
        "Summary".to_string(),
        Metric::Memory.format(s.min),
        Metric::Memory.format(s.max),
        Metric::Memory.format(s.mean as u64),
        Metric::Memory.format(s.p95),
        s.count.to_string(),
        Metric::Memory.format(s.sum),
    ]);
    println!("{table}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(resource: &str, sum: u64) -> StatsRow {
        StatsRow {
            resource: resource.to_string(),
            stats: Statistics {
                min: sum,
                max: sum,
                mean: sum as f64,
                p95: sum,
                count: 1,
                sum,
            },
        }
    }

    #[test]
    fn sorts_by_integer_bytes_not_human_string() {
        // 512 MiB vs 1.48 GiB: as human strings "1.48 GiB" < "512 MiB",
        // but by raw bytes the GiB value must come first when descending.
        let mib_512 = 512 * 1024 * 1024; // 536,870,912
        let gib_1_48 = 1_589_137_899; // ~1.48 GiB
        let mut rows = vec![row("a/mib", mib_512), row("b/gib", gib_1_48)];

        sort_rows(&mut rows, 6, true); // sort by sum, descending
        assert_eq!(rows[0].resource, "b/gib");
        assert_eq!(rows[1].resource, "a/mib");

        sort_rows(&mut rows, 6, false); // ascending
        assert_eq!(rows[0].resource, "a/mib");
        assert_eq!(rows[1].resource, "b/gib");
    }

    #[test]
    fn sorts_resource_column_alphabetically() {
        let mut rows = vec![row("z/one", 1), row("a/two", 2)];
        sort_rows(&mut rows, 0, false);
        assert_eq!(rows[0].resource, "a/two");
    }
}
