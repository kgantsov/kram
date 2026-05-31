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
  -n, --namespace <NAMESPACE>      Namespace to query (default: all namespaces)
  -s, --selector <SELECTOR>        Label selector (repeatable; see below)
      --sort-by <SORT_BY>          Column to sort by [default: sum]
                                   [possible values: resource, min, max, mean, p95, count, sum]
      --sort-order <SORT_ORDER>    Sort direction [default: desc]
                                   [possible values: asc, desc]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Examples

```bash
# All namespaces
kram

# Single namespace
kram -n production

# Filter by label (AND within a selector)
kram --selector app=nginx,tier=web

# Filter by multiple labels with OR logic (repeat --selector)
kram --selector app=nginx --selector app=redis

# Combined: (app=nginx AND tier=web) OR (app=redis)
kram --selector app=nginx,tier=web --selector app=redis

# Sort by max memory, ascending
kram --sort-by max --sort-order asc

# Sort by p95, descending (default order)
kram --sort-by p95
```

### Label selectors

`--selector` accepts comma-separated `key=value` pairs that must **all** match (AND). Repeating `--selector` adds an OR group — a pod matches if it satisfies **any** of the selector groups.

| Example | Meaning |
|---|---|
| `--selector app=nginx` | `app=nginx` |
| `--selector app=nginx,tier=web` | `app=nginx` AND `tier=web` |
| `--selector app=nginx --selector app=redis` | `app=nginx` OR `app=redis` |
| `--selector app=nginx,tier=web --selector app=redis` | (`app=nginx` AND `tier=web`) OR `app=redis` |

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

Rows are sorted by `sum` descending by default. Use `--sort-by` to change the column and `--sort-order` to change direction. The `Summary` row aggregates across all containers.

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
