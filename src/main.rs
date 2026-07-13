use clap::Parser;
use log::{error, info};
use petrify::config::Config;
use petrify::engine::Petrifier;

#[derive(Parser)]
#[command(name = "petrify")]
#[command(about = "Petrify live websites into static offline copies")]
#[command(version)]
struct Cli {
    /// Target URL to petrify
    #[arg(required = true)]
    url: String,

    /// Maximum crawling depth
    #[arg(short, long, default_value = "unlimited")]
    depth: String,

    /// Maximum number of pages to scan and process (for testing)
    #[arg(long, default_value_t = 0)]
    max_pages: usize,

    /// Download only specific resource types
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "js,css,images,video,html,pdf,fonts"
    )]
    download_only: Vec<String>,

    /// Maximum concurrent workers
    #[arg(short, long, default_value_t = num_cpus::get())]
    max_concurrent: usize,

    /// Output directory for the petrified site
    #[arg(short, long, default_value = "./petrified_site")]
    output: String,

    /// Download external resources
    #[arg(long, default_value_t = true)]
    download_external: bool,

    /// Convert JPEG/PNG images to WebP format
    #[arg(long, default_value_t = true)]
    convert_to_webp: bool,

    /// WebP quality (1-100)
    #[arg(long, default_value_t = 75)]
    webp_quality: u8,

    /// Use lossless WebP compression
    #[arg(long, default_value_t = false)]
    webp_lossless: bool,

    /// Ignore robots.txt restrictions
    #[arg(short, long, default_value_t = true)]
    ignore_robots: bool,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 270)]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    info!("Starting petrify");
    info!("Target URL: {}", cli.url);
    info!("Output directory: {}", cli.output);

    let config = Config {
        url: cli.url,
        depth: cli.depth,
        max_pages: cli.max_pages,
        download_only: cli.download_only,
        max_concurrent: cli.max_concurrent,
        output: cli.output,
        download_external: cli.download_external,
        convert_to_webp: cli.convert_to_webp,
        webp_quality: cli.webp_quality,
        webp_lossless: cli.webp_lossless,
        ignore_robots: cli.ignore_robots,
        timeout: cli.timeout,
    };

    let mut petrifier = Petrifier::new(config).await?;

    match petrifier.run().await {
        Ok(_) => {
            info!("Petrify completed successfully!");
            Ok(())
        }
        Err(e) => {
            error!("Petrify failed: {}", e);
            Err(e.into())
        }
    }
}
