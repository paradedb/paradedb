mod cli;
mod subcommand;
mod tables;

use anyhow::Result;
use async_std::task::block_on;
use cli::{Cli, Corpus, EsLogsCommand, Subcommand};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::default();
    match cli.subcommand {
        Subcommand::Install => subcommand::install(),
        Subcommand::Bench(bench) => match bench.corpus {
            Corpus::Eslogs(eslogs) => match eslogs.command {
                EsLogsCommand::Generate {
                    seed,
                    events,
                    table,
                    url,
                } => block_on(subcommand::bench_eslogs_generate(seed, events, table, url)),
                EsLogsCommand::BuildSearchIndex {
                    table,
                    index_name,
                    url,
                } => block_on(subcommand::bench_eslogs_build_search_index(
                    table, index_name, url,
                )),
                EsLogsCommand::QuerySearchIndex { .. } => Ok(()),
            },
        },
    }
}
