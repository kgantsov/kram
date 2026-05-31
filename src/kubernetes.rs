use k8s_openapi::api::apps::v1::ReplicaSet;
use k8s_openapi::api::core::v1::Pod;
use kube::api::ObjectList;
use kube::core::DynamicObject;
use kube::discovery::ApiResource;
use kube::{Api, Client, api::ListParams};

pub async fn get_pods(
    client: Client,
    namespace: Option<String>,
    selectors: Vec<String>,
) -> anyhow::Result<ObjectList<Pod>> {
    let pods: Api<Pod> = match namespace {
        Some(ns) => Api::namespaced(client, ns.as_str()),
        None => Api::all(client),
    };

    let pod_list = pods.list(&Default::default()).await?;

    let pod_list: ObjectList<Pod> = if selectors.is_empty() {
        pod_list
    } else {
        let metadata = pod_list.metadata.clone();
        let types = pod_list.types.clone();
        let items = pod_list
            .into_iter()
            .filter(|pod| {
                let Some(labels) = &pod.metadata.labels else {
                    return false;
                };
                // OR across groups; AND within each comma-separated group
                selectors.iter().any(|group| {
                    group.split(',').all(|s| {
                        let mut parts = s.splitn(2, '=');
                        let key = parts.next().unwrap_or("");
                        let value = parts.next().unwrap_or("");
                        labels.get(key).map(|v| v == value).unwrap_or(false)
                    })
                })
            })
            .collect();
        ObjectList {
            metadata,
            items,
            types,
        }
    };

    Ok(pod_list)
}

pub async fn get_metrics(
    client: Client,
    namespace: Option<String>,
) -> anyhow::Result<ObjectList<DynamicObject>> {
    let ar = ApiResource {
        group: "metrics.k8s.io".into(),
        version: "v1beta1".into(),
        api_version: "metrics.k8s.io/v1beta1".into(),
        kind: "PodMetrics".into(),
        plural: "pods".into(),
    };

    let api: Api<DynamicObject> = match namespace {
        Some(ns) => Api::namespaced_with(client, ns.as_str(), &ar),
        None => Api::all_with(client, &ar),
    };
    let metrics = api.list(&ListParams::default()).await?;

    Ok(metrics)
}

pub async fn resolve_pod_owner(client: Client, pod: &Pod) -> anyhow::Result<String> {
    let refs = pod
        .metadata
        .owner_references
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Pod has no owner references"))?;

    let pod_ns = pod.metadata.namespace.as_deref().unwrap_or("default");
    let rs_api: Api<ReplicaSet> = Api::namespaced(client.clone(), pod_ns);

    for oref in refs {
        match oref.kind.as_str() {
            "ReplicaSet" => {
                let rs = rs_api.get(&oref.name).await?;
                if let Some(d) = rs
                    .metadata
                    .owner_references
                    .as_ref()
                    .and_then(|refs| refs.iter().find(|r| r.kind == "Deployment"))
                {
                    return Ok(format!("deployment/{}", d.name));
                }
            }
            "StatefulSet" => return Ok(format!("statefulset/{}", oref.name)),
            "DaemonSet" => return Ok(format!("daemonset/{}", oref.name)),
            "Job" => return Ok(format!("job/{}", oref.name)),
            _ => {}
        }
    }
    Ok(format!(
        "standalone/{}",
        pod.metadata.name.as_deref().unwrap_or("unknown")
    ))
}
