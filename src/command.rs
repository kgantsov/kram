use clap::Parser;

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
}
