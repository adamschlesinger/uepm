use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "uepm", version, about = "Unreal Engine Package Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a project for UEPM
    Init {
        /// Accept detected defaults without prompting
        #[arg(short, long)]
        yes: bool,
    },
    /// Install one or all plugins
    Install {
        /// Packages to install, e.g. @scope/plugin or @scope/plugin@1.0.0
        /// If empty, installs all plugins in uepm.ini
        packages: Vec<String>,
    },
    /// Remove a plugin
    Uninstall { package: String },
    /// Update plugins to latest compatible versions
    Update {
        /// Specific package to update; updates all if omitted
        package: Option<String>,
    },
    /// List installed plugins and compatibility status
    List,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes } => uepm::commands::init::run(yes).await,
        Commands::Install { packages } => uepm::commands::install::run(packages).await,
        Commands::Uninstall { package } => uepm::commands::uninstall::run(package).await,
        Commands::Update { package } => uepm::commands::update::run(package).await,
        Commands::List => uepm::commands::list::run().await,
    };

    if let Err(e) = result {
        uepm::output::print_error(&format!("{e}"));
        std::process::exit(1);
    }
}
