use clap::Parser;
use kram::run::run;

use kram::command::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    run(cli.namespace).await?;

    Ok(())
}
