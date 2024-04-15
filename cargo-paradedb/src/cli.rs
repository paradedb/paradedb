use clap::Parser;

const DEFAULT_BENCH_ESLOGS_TABLE: &str = "benchmark_eslogs";
const DEFAULT_BENCH_ESLOGS_INDEX_NAME: &str = "benchmark_eslogs_pg_search";

/// A wrapper struct for subcommands.
#[derive(Parser)]
#[command(version, about, long_about = None, bin_name = "cargo")]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

// Top-level commands for the cargo-paradedb tool.
#[derive(clap::Subcommand)]
pub enum Subcommand {
    Install,
    Bench(CorpusArgs),
}

// A wrapper struct for a subcommand under 'cargo paradedb bench' which
// select a corpus to generate/run.
#[derive(Debug, clap::Args)]
pub struct CorpusArgs {
    #[command(subcommand)]
    pub corpus: Corpus,
}

/// Which benchmark dataset to run or generate.
#[derive(Debug, clap::Subcommand)]
pub enum Corpus {
    // The generated logs from the ElasticSearch benchmark tool.
    Eslogs(EsLogsArgs),
}

/// A wrapper struct for the command to run on the eslogs corpus.
#[derive(Debug, clap::Args)]
pub struct EsLogsArgs {
    #[command(subcommand)]
    pub command: EsLogsCommand,
}

/// The command to run on the eslogs corpus.
#[derive(Debug, clap::Subcommand)]
pub enum EsLogsCommand {
    /// Generate the eslogs corpus, inserting into a Postgres table.
    Generate {
        /// Starting seed for random generation.
        #[arg(long, short, default_value_t = 1)]
        seed: u64,
        /// Total number of events to generate per file.
        /// Defaults to a file size of 100MB.
        #[arg(long, short, default_value_t = 118891)]
        events: u64,
        /// Postgres table name to insert into.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildSearchIndex {
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_INDEX_NAME)]
        index: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    QuerySearchIndex {
        /// Postgres index name to query.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_INDEX_NAME)]
        index: String,
        /// Query to run.
        #[arg(short, long, default_value = "message:flame")]
        query: String,
        /// Limit results to return.
        #[arg(short, long, default_value_t = 1)]
        limit: u64,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildParquetTable {
        /// Postgres table name to build from.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    CountParquetTable {
        /// Postgres table name to build from.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
}

impl Default for Cli {
    fn default() -> Self {
        // Usually, clap CLI tools can just use `Self::parse()` to initialize a
        // struct with the CLI arguments... but seeing as this will be run as a
        // cargo subcommand, we need to do some extra work.
        //
        // Because we're running e.g. "cargo paradedb install"... clap will think
        // that "paradedb" is the first argument that was passed to the binary.
        // Instead we want "install" to be the first argument, with "paradedb"
        // ignored. For this reason, we'll manually collect and process the CLI
        // arguments ourselves.
        //
        // We check to see if the argument at index 1 is "paradedb"...
        // as the argument at index 0 is always the path to the binary executable.
        // If "paradedb" is found, we'll parse the arguments starting at index 1.
        // Otherwise, we'll use Self::parse() like usual.
        let args = std::env::args().collect::<Vec<String>>();
        match args.get(1) {
            Some(arg) if arg == "paradedb" => Self::parse_from(&args[1..]),
            _ => Self::parse(),
        }
    }
}
