use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SortBy {
    Resource,
    Min,
    Max,
    Mean,
    P95,
    Count,
    Sum,
}

#[derive(Parser, Debug)]
#[command(
    name = "kram",
    version,
    about = "A simple tool for querying Kubernetes pod metrics",
    long_about = "Kram is a command-line tool that allows you to query Kubernetes pod metrics such as CPU and memory usage. It can be used to quickly check the resource usage of your pods without needing to set up complex monitoring solutions."
)]
pub struct Cli {
    /// Namespace to query for pods and metrics (default: all namespaces)
    #[arg(short, long)]
    pub namespace: Option<String>,

    /// Sort order
    #[arg(long = "sort-order", default_value = "desc")]
    pub sort_order: SortOrder,

    /// Sort output by a specific column (e.g., "resource", "min", "max", "mean", "p95", "count", "sum"). Default is "sum"
    #[arg(long = "sort-by", default_value = "sum")]
    pub sort_by: SortBy,

    /// Label selector to filter pods (e.g., "app=nginx,tier=web"); repeat for OR logic
    #[arg(short, long)]
    pub selector: Vec<String>,
}
