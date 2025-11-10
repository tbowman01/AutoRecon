use autorecon_core::{
    config::Config,
    scanner::{ScanContext, Scanner, Target},
    Result,
};
use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::time::Instant;

mod executor;
mod output;

use executor::NativeExecutor;
use output::NativeOutputHandler;

/// AutoRecon - Network reconnaissance automation tool
#[derive(Parser, Debug)]
#[command(name = "autorecon")]
#[command(author = "Tib3rius")]
#[command(version = "1.0.0")]
#[command(about = "Multi-threaded network reconnaissance automation", long_about = None)]
struct Args {
    /// Target IP addresses or hostnames
    #[arg(required = true)]
    targets: Vec<String>,

    /// Output directory for results
    #[arg(short = 'o', long, default_value = "results")]
    output: PathBuf,

    /// Port scan profile to use
    #[arg(long, default_value = "default")]
    profile: String,

    /// Number of targets to scan concurrently
    #[arg(short = 't', long = "concurrent-targets", default_value = "5")]
    concurrent_targets: usize,

    /// Number of scans per target to run concurrently
    #[arg(short = 's', long = "concurrent-scans", default_value = "10")]
    concurrent_scans: usize,

    /// Configuration directory
    #[arg(short = 'c', long = "config", default_value = "config")]
    config_dir: PathBuf,

    /// Verbosity level (0-2)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbosity: u8,

    /// Heartbeat interval in seconds
    #[arg(long = "heartbeat", default_value = "60")]
    heartbeat: u64,

    /// Additional nmap arguments
    #[arg(long = "nmap")]
    nmap: Option<String>,

    /// Append to default nmap arguments
    #[arg(long = "nmap-append")]
    nmap_append: Option<String>,

    /// Only scan a single target
    #[arg(long)]
    single_target: bool,

    /// List available profiles and exit
    #[arg(long)]
    list_profiles: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();

    // Print banner
    print_banner();

    // Load configuration
    let config = Config::load_from_dir(&args.config_dir)?;

    // List profiles if requested
    if args.list_profiles {
        list_profiles(&config);
        return Ok(());
    }

    // Validate profile exists
    if !config.port_scan_profiles.contains_key(&args.profile) {
        eprintln!(
            "{} Profile '{}' not found. Available profiles:",
            "Error:".red().bold(),
            args.profile
        );
        for profile_name in config.port_scan_profiles.keys() {
            eprintln!("  - {}", profile_name);
        }
        std::process::exit(1);
    }

    // Sanity check for number of targets
    if args.targets.len() > 256 && !args.single_target {
        eprintln!(
            "{} You specified {} targets. Are you sure you want to scan this many?",
            "Warning:".yellow().bold(),
            args.targets.len()
        );
        eprintln!("Use --single-target flag to scan only the first target, or reduce the number of targets.");
        eprintln!("Press Ctrl+C to cancel, or wait 10 seconds to continue...");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }

    // Determine nmap arguments
    let nmap_extra = if let Some(nmap) = args.nmap {
        nmap
    } else if let Some(append) = args.nmap_append {
        format!("-vv --reason -Pn {}", append)
    } else {
        "-vv --reason -Pn".to_string()
    };

    // Create scan context
    let context = ScanContext {
        concurrent_targets: args.concurrent_targets,
        concurrent_scans: args.concurrent_scans,
        profile: args.profile.clone(),
        output_dir: args.output.clone(),
        verbosity: args.verbosity,
        heartbeat_interval: args.heartbeat,
        nmap_extra,
    };

    // Create executor and output handler
    let executor = NativeExecutor::new(context.concurrent_scans);
    let output_handler = NativeOutputHandler::new(context.verbosity);

    // Create scanner
    let scanner = Scanner::new(config, executor, output_handler, context.clone());

    // Determine targets to scan
    let targets_to_scan = if args.single_target {
        vec![args.targets[0].clone()]
    } else {
        args.targets.clone()
    };

    println!(
        "{} Scanning {} target(s) with profile '{}'",
        "Info:".blue().bold(),
        targets_to_scan.len(),
        args.profile.green()
    );
    println!(
        "{} Output directory: {}",
        "Info:".blue().bold(),
        args.output.display()
    );
    println!(
        "{} Concurrent targets: {}, Concurrent scans per target: {}",
        "Info:".blue().bold(),
        context.concurrent_targets,
        context.concurrent_scans
    );
    println!();

    // Start scanning
    let start_time = Instant::now();

    // Scan targets concurrently
    let mut handles = Vec::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(context.concurrent_targets));

    for target_address in targets_to_scan {
        let sem = semaphore.clone();
        let scanner = scanner.clone();
        let output_dir = context.output_dir.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            println!(
                "{} [{}] Starting scan",
                "-->".green().bold(),
                target_address.cyan()
            );

            let mut target = Target::new(target_address.clone(), &output_dir);

            match scanner.scan_target(&mut target).await {
                Ok(_) => {
                    println!(
                        "{} [{}] Scan completed - {} service(s) detected",
                        "<--".green().bold(),
                        target_address.cyan(),
                        target.services.len()
                    );
                    Ok(target)
                }
                Err(e) => {
                    eprintln!(
                        "{} [{}] Scan failed: {}",
                        "!!!".red().bold(),
                        target_address.cyan(),
                        e
                    );
                    Err(e)
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all scans to complete
    let mut completed = 0;
    let mut failed = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(_target)) => completed += 1,
            Ok(Err(_e)) => failed += 1,
            Err(e) => {
                eprintln!("{} Task panicked: {}", "Error:".red().bold(), e);
                failed += 1;
            }
        }
    }

    let elapsed = start_time.elapsed();

    println!();
    println!("{}", "=".repeat(60).bright_black());
    println!(
        "{} Scan Summary",
        "Summary:".bright_cyan().bold()
    );
    println!("{}", "=".repeat(60).bright_black());
    println!("  Completed: {} target(s)", completed.to_string().green());
    println!("  Failed:    {} target(s)", failed.to_string().red());
    println!("  Duration:  {:.2}s", elapsed.as_secs_f64());
    println!("{}", "=".repeat(60).bright_black());

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_banner() {
    let banner = r#"
   ___        __       ____
  / _ | __ __/ /_ ___ / __ \___  _______  ___
 / __ |/ // / __/ _ \/ /_/ / -_)/ __/ _ \/ _ \
/_/ |_|\_,_/\__/\___/_  _ /\__/ \__/\___/_//_/
                     /_//_/
"#;
    println!("{}", banner.bright_cyan());
    println!("Network reconnaissance automation");
    println!("by Tib3rius | Rust Edition v1.0.0");
    println!();
}

fn list_profiles(config: &Config) {
    println!("{}", "Available Scan Profiles:".bright_cyan().bold());
    println!();

    for (name, profile) in &config.port_scan_profiles {
        println!("  {} {}", "•".bright_blue(), name.green().bold());
        println!("    {} scans:", profile.scans.len());

        for scan in &profile.scans {
            println!("      - {}", scan.name.bright_black());
        }
        println!();
    }
}
