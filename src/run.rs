use comfy_table::Table;
use k8s_openapi::api::apps::v1::ReplicaSet;
use std::collections::HashMap;

use crate::command::{SortBy, SortOrder};
use crate::kubernetes::{get_metrics, get_pods, get_replicasets, resolve_pod_owner};
use crate::metrics::{
    PodMetrics, Statistics, calculate_stats, memory_bytes_to_human, parse_memory_bytes,
};
use kube::Client;

pub async fn run(
    namespace: Option<String>,
    selector: Vec<String>,
    sort_order: SortOrder,
    sort_by: SortBy,
) -> anyhow::Result<()> {
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
    let mut owner_metrics: HashMap<String, Vec<u64>> = HashMap::new();

    for metric in &metrics_list {
        let pod_name = metric.metadata.name.clone().unwrap_or_default();
        let pod_owner = owner_map.get(&pod_name).cloned();

        let pod_owner = match pod_owner {
            Some(owner) => owner,
            None => {
                continue;
            }
        };

        let pod_metrics: PodMetrics = serde_json::from_value(serde_json::to_value(metric)?)?;
        for container in &pod_metrics.containers {
            let memory_bytes = parse_memory_bytes(&container.usage.memory);
            owner_metrics
                .entry(format!("{pod_owner}/{}", container.name))
                .or_default()
                .push(memory_bytes);
        }
    }
    let mut mem_usage_values: Vec<u64> = Vec::new();
    let mut owner_stats: HashMap<String, Statistics> = HashMap::new();
    for (owner, values) in &owner_metrics {
        let (min, max, mean, p95, count, sum) = calculate_stats(values.clone());
        owner_stats.insert(
            owner.clone(),
            Statistics {
                min,
                max,
                mean,
                p95,
                count,
                sum,
            },
        );
        mem_usage_values.extend(values);
    }

    let mut table = Table::new();
    table.set_header(vec![
        "Resource", "min", "max", "mean", "p95", "count", "sum",
    ]);

    let mut sorted_stats: Vec<(String, Statistics)> = owner_stats.into_iter().collect();
    sorted_stats.sort_by_key(|(_, stats)| match sort_by {
        SortBy::Min => stats.min,
        SortBy::Max => stats.max,
        SortBy::Mean => stats.mean as u64,
        SortBy::P95 => stats.p95,
        SortBy::Count => stats.count as u64,
        SortBy::Sum => stats.sum,
    });
    if matches!(sort_order, SortOrder::Desc) {
        sorted_stats.reverse();
    }

    for (owner, stats) in sorted_stats {
        table.add_row(vec![
            owner.clone(),
            memory_bytes_to_human(stats.min),
            memory_bytes_to_human(stats.max),
            memory_bytes_to_human(stats.mean as u64),
            memory_bytes_to_human(stats.p95),
            stats.count.to_string(),
            memory_bytes_to_human(stats.sum),
        ]);
    }

    let (min, max, mean, p95, count, sum) = calculate_stats(mem_usage_values);

    table.add_row(vec![
        "Summary".to_string(),
        memory_bytes_to_human(min),
        memory_bytes_to_human(max),
        memory_bytes_to_human(mean as u64),
        memory_bytes_to_human(p95),
        count.to_string(),
        memory_bytes_to_human(sum),
    ]);
    println!("{table}");

    Ok(())
}
