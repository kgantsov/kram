# kram

A Kubernetes memory usage CLI. Queries pod metrics and prints aggregated statistics — min, max, mean, p95, count, and sum — grouped by workload owner (Deployment, StatefulSet, DaemonSet, Job, or standalone pod) per container.

## Requirements

- Rust (edition 2024)
- A Kubernetes cluster reachable via `~/.kube/config` or `KUBECONFIG`
- [metrics-server](https://github.com/kubernetes-sigs/metrics-server) running in the cluster

## Usage

```
kram [OPTIONS]

Options:
  -n, --namespace <NAMESPACE>  Namespace to query (default: all namespaces)
  -h, --help                   Print help
  -V, --version                Print version
```

### Examples

```bash
# All namespaces
kram

# Single namespace
kram -n production
```

### Output

```
+----------------------------------------------+--------+--------+--------+--------+-------+--------+
| Resource                                     | min    | max    | mean   | p95    | count | sum    |
+----------------------------------------------+--------+--------+--------+--------+-------+--------+
| deployment/api-server/app                    | 128 MiB| 256 MiB| 192 MiB| 248 MiB| 3     | 576 MiB|
| statefulset/postgres/postgres                | 512 MiB| 512 MiB| 512 MiB| 512 MiB| 1     | 512 MiB|
| ...                                          |        |        |        |        |       |        |
+----------------------------------------------+--------+--------+--------+--------+-------+--------+
| Summary                                      | ...    | ...    | ...    | ...    | N     | ...    |
+----------------------------------------------+--------+--------+--------+--------+-------+--------+
```

Rows are sorted descending by total memory. The `Summary` row aggregates across all containers.

## Build

```bash
cargo build          # debug binary → target/debug/kram
cargo build --release  # release binary → target/release/kram
```

## Development

```bash
cargo run            # build and run against current kubeconfig
cargo test           # run tests
cargo clippy         # lint
cargo fmt            # format
```
