//! Node.js native bindings for AutoRecon using napi-rs
//!
//! This module provides zero-copy native Node.js bindings for AutoRecon,
//! enabling high-performance network reconnaissance from JavaScript/TypeScript.

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction};
use napi_derive::napi;
use std::path::PathBuf;
use std::sync::Arc;

mod executor;
mod output;

use executor::NodeExecutor;
use output::NodeOutputHandler;

/// Configuration options for AutoRecon scanner
#[napi(object)]
#[derive(Clone)]
pub struct AutoReconConfig {
    /// Configuration directory path containing TOML files
    pub config_dir: String,

    /// Port scan profile to use (e.g., "default", "quick", "udp")
    pub profile: String,

    /// Number of targets to scan concurrently
    pub concurrent_targets: Option<u32>,

    /// Number of scans to run per target concurrently
    pub concurrent_scans: Option<u32>,

    /// Output directory for scan results
    pub output_dir: String,

    /// Verbosity level (0-2)
    pub verbosity: Option<u32>,

    /// Heartbeat interval in seconds
    pub heartbeat_interval: Option<u32>,

    /// Additional nmap arguments
    pub nmap_extra: Option<String>,
}

impl Default for AutoReconConfig {
    fn default() -> Self {
        AutoReconConfig {
            config_dir: "config".to_string(),
            profile: "default".to_string(),
            concurrent_targets: Some(5),
            concurrent_scans: Some(10),
            output_dir: "results".to_string(),
            verbosity: Some(1),
            heartbeat_interval: Some(60),
            nmap_extra: Some("-vv --reason -Pn".to_string()),
        }
    }
}

/// Detected service from port scanning
#[napi(object)]
#[derive(Clone)]
pub struct DetectedService {
    /// Protocol (tcp/udp)
    pub protocol: String,

    /// Port number
    pub port: u32,

    /// Service name
    pub service: String,

    /// Optional version information
    pub version: Option<String>,
}

/// Scan target result
#[napi(object)]
pub struct ScanResult {
    /// Target address
    pub address: String,

    /// Detected services
    pub services: Vec<DetectedService>,

    /// Base directory for results
    pub base_dir: String,

    /// Scan duration in milliseconds
    pub duration_ms: u32,

    /// Whether the scan completed successfully
    pub success: bool,

    /// Error message if scan failed
    pub error: Option<String>,
}

/// Main AutoRecon scanner class
#[napi]
pub struct AutoRecon {
    config: autorecon_core::config::Config,
    scan_context: autorecon_core::scanner::ScanContext,
}

#[napi]
impl AutoRecon {
    /// Create a new AutoRecon instance
    #[napi(constructor)]
    pub fn new(config: AutoReconConfig) -> Result<Self> {
        // Load configuration from TOML files
        let core_config = autorecon_core::config::Config::load_from_dir(
            &PathBuf::from(&config.config_dir),
        )
        .map_err(|e| Error::from_reason(format!("Failed to load config: {}", e)))?;

        // Validate profile exists
        if !core_config.port_scan_profiles.contains_key(&config.profile) {
            return Err(Error::from_reason(format!(
                "Profile '{}' not found",
                config.profile
            )));
        }

        // Create scan context
        let scan_context = autorecon_core::scanner::ScanContext {
            concurrent_targets: config.concurrent_targets.unwrap_or(5) as usize,
            concurrent_scans: config.concurrent_scans.unwrap_or(10) as usize,
            profile: config.profile.clone(),
            output_dir: PathBuf::from(&config.output_dir),
            verbosity: config.verbosity.unwrap_or(1) as u8,
            heartbeat_interval: config.heartbeat_interval.unwrap_or(60) as u64,
            nmap_extra: config
                .nmap_extra
                .clone()
                .unwrap_or_else(|| "-vv --reason -Pn".to_string()),
        };

        Ok(AutoRecon {
            config: core_config,
            scan_context,
        })
    }

    /// Scan a single target
    ///
    /// This method takes a threadsafe callback function for command execution.
    /// The callback will be called from a background thread.
    #[napi(ts_return_type = "Promise<ScanResult>")]
    pub fn scan_target(
        &self,
        address: String,
        execute_callback: JsFunction,
    ) -> Result<AsyncTask<ScanTask>> {
        let tsfn: ThreadsafeFunction<String, ErrorStrategy::CalleeHandled> = execute_callback
            .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<String>| {
                Ok(vec![ctx.value])
            })?;

        Ok(AsyncTask::new(ScanTask {
            config: self.config.clone(),
            scan_context: self.scan_context.clone(),
            address,
            execute_fn: tsfn,
        }))
    }

    /// List available scan profiles
    #[napi]
    pub fn list_profiles(&self) -> Vec<String> {
        self.config
            .port_scan_profiles
            .keys()
            .cloned()
            .collect()
    }

    /// Get profile information
    #[napi]
    pub fn get_profile_info(&self, profile_name: String) -> Option<ProfileInfo> {
        self.config
            .port_scan_profiles
            .get(&profile_name)
            .map(|profile| ProfileInfo {
                name: profile_name,
                scan_count: profile.scans.len() as u32,
                scans: profile.scans.iter().map(|s| s.name.clone()).collect(),
            })
    }
}

/// Async task for scanning a target
pub struct ScanTask {
    config: autorecon_core::config::Config,
    scan_context: autorecon_core::scanner::ScanContext,
    address: String,
    execute_fn: ThreadsafeFunction<String, ErrorStrategy::CalleeHandled>,
}

impl Task for ScanTask {
    type Output = ScanResult;
    type JsValue = ScanResult;

    fn compute(&mut self) -> Result<Self::Output> {
        let start = std::time::Instant::now();

        // Create tokio runtime for this task
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::from_reason(format!("Failed to create runtime: {}", e)))?;

        // Run scan in the runtime
        let result = runtime.block_on(async {
            // Create executor with threadsafe callback
            let executor = NodeExecutor::new(
                self.scan_context.concurrent_scans,
                Arc::new(self.execute_fn.clone()),
            );

            // Create output handler
            let output_handler = NodeOutputHandler::new(self.scan_context.verbosity);

            // Create scanner
            let scanner = autorecon_core::scanner::Scanner::new(
                self.config.clone(),
                executor,
                output_handler,
                self.scan_context.clone(),
            );

            // Create target
            let mut target = autorecon_core::scanner::Target::new(
                self.address.clone(),
                &self.scan_context.output_dir,
            );

            // Run scan
            let scan_result = scanner.scan_target(&mut target).await;

            (target, scan_result)
        });

        let duration = start.elapsed();
        let (target, scan_result) = result;

        match scan_result {
            Ok(_) => {
                // Convert services to N-API format
                let services = target
                    .services
                    .into_iter()
                    .map(|s| DetectedService {
                        protocol: s.protocol.as_str().to_string(),
                        port: s.port as u32,
                        service: s.service,
                        version: s.version,
                    })
                    .collect();

                Ok(ScanResult {
                    address: self.address.clone(),
                    services,
                    base_dir: target.base_dir.to_string_lossy().to_string(),
                    duration_ms: duration.as_millis() as u32,
                    success: true,
                    error: None,
                })
            }
            Err(e) => Ok(ScanResult {
                address: self.address.clone(),
                services: vec![],
                base_dir: target.base_dir.to_string_lossy().to_string(),
                duration_ms: duration.as_millis() as u32,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

/// Profile information
#[napi(object)]
pub struct ProfileInfo {
    /// Profile name
    pub name: String,

    /// Number of scans in profile
    pub scan_count: u32,

    /// List of scan names
    pub scans: Vec<String>,
}

/// Initialize the module
#[napi]
pub fn init() {
    // Module initialization if needed
}
