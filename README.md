# kram

An interactive Kubernetes resource-usage TUI. Queries pod metrics and shows aggregated statistics — min, max, mean, p95, count, and sum — grouped by workload owner (Deployment, StatefulSet, DaemonSet, Job, or standalone pod) per container. Switch between **memory** and **CPU** views, sort by any column, and filter workloads live — all in memory, with a single query to the cluster on startup.

![Screenshot-6](/.github/screenshots/Screenshot-1.png)

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
      --sort-by <SORT_BY>          Column to sort by initially [default: sum]
                                   [possible values: min, max, mean, p95, count, sum]
      --sort-order <SORT_ORDER>    Initial sort direction [default: desc]
                                   [possible values: asc, desc]
  -h, --help                       Print help
  -V, --version                    Print version
```

`--sort-by` and `--sort-order` set the initial ordering; both can be changed interactively once the TUI is running.

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

# Start sorted by max, ascending
kram --sort-by max --sort-order asc

# Start sorted by p95, descending (default order)
kram --sort-by p95
```

## Interactive controls

The cluster is queried once on startup; all interaction below is instant and never re-queries.

| Key | Action |
|---|---|
| `↑` / `↓` or `k` / `j` | Move the row selection |
| `g` / `G` | Jump to the first / last row |
| `←` / `→` or `h` / `l` | Move the active sort column |
| `s` / `Space` / `Enter` | Toggle ascending / descending |
| `m` | Switch between memory and CPU views |
| `/` | Filter workloads by resource name |
| `q` | Quit |

### Filtering

Press `/` to open the filter prompt and type to match against the resource label (`<kind>/<name>/<container>`, case-insensitive) — the table updates as you type.

- `Enter` applies the filter and returns to navigation (the filter stays active and is shown at the bottom-left of the table).
- `Esc` clears the filter. In normal mode, `Esc` clears an active filter first and only quits when none is set; `q` always quits.

### Label selectors

`--selector` accepts comma-separated `key=value` pairs that must **all** match (AND). Repeating `--selector` adds an OR group — a pod matches if it satisfies **any** of the selector groups.

| Example | Meaning |
|---|---|
| `--selector app=nginx` | `app=nginx` |
| `--selector app=nginx,tier=web` | `app=nginx` AND `tier=web` |
| `--selector app=nginx --selector app=redis` | `app=nginx` OR `app=redis` |
| `--selector app=nginx,tier=web --selector app=redis` | (`app=nginx` AND `tier=web`) OR `app=redis` |

### Views

Rows are grouped as `<owner-kind>/<owner-name>/<container>` and start sorted by `sum` descending. Memory is shown in binary units (KiB/MiB/GiB), CPU in millicores below one core and cores above. The `Summary` footer aggregates across the currently visible (filtered) rows.

## Build

```bash
cargo build            # debug binary → target/debug/kram
cargo build --release  # release binary → target/release/kram
```

## Development

```bash
cargo run            # build and run against current kubeconfig
cargo test           # run tests
cargo clippy         # lint
cargo fmt            # format
```
