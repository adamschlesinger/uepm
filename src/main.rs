use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use uepm::context::UEPMContext;

#[derive(Parser)]
#[command(name = "uepm", version, about = "Unreal Engine Plugin Manager")]
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
        /// If empty, installs all plugins in Config/UEPM.ini
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
    /// Publish this plugin to the npm registry
    Publish {
        /// npm dist-tag (default: latest)
        #[arg(long, default_value = "latest")]
        tag: String,
        /// Validate and build tarball but do not upload
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
        /// Registry access level
        #[arg(long, default_value = "public")]
        access: String,
    },
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let cli = Cli::parse();

    let ctx = match UEPMContext::new() {
        Ok(c) => c,
        Err(e) => {
            uepm::output::print_error(&format!("{e}"));
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        Commands::Init { yes } => uepm::commands::init::run(&ctx, yes).await,
        Commands::Install { packages } => uepm::commands::install::run(&ctx, packages).await,
        Commands::Uninstall { package } => uepm::commands::uninstall::run(&ctx, package).await,
        Commands::Update { package } => uepm::commands::update::run(&ctx, package).await,
        Commands::List => uepm::commands::list::run(&ctx).await,
        Commands::Publish { tag, dry_run, yes, access } => {
            uepm::commands::publish::run(&ctx, &tag, dry_run, yes, &access).await
        }
    };

    if let Err(e) = result {
        uepm::output::print_error(&format!("{e}"));
        std::process::exit(1);
    }
}
