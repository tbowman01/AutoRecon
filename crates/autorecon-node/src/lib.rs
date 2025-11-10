//! Node.js native bindings for AutoRecon using napi-rs
//!
//! This module provides zero-copy native Node.js bindings for AutoRecon,
//! enabling high-performance network reconnaissance from JavaScript/TypeScript.

use napi::bindgen_prelude::*;
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
    runtime: tokio::runtime::Runtime,
}

#[napi]
impl AutoRecon {
    /// Create a new AutoRecon instance
    ///
    /// # Arguments
    /// * `config` - Configuration options
    ///
    /// # Example
    /// ```javascript
    /// const autorecon = new AutoRecon({
    ///   configDir: './config',
    ///   profile: 'default',
    ///   outputDir: './results'
    /// });
    /// ```
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

        // Create tokio runtime
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::from_reason(format!("Failed to create runtime: {}", e)))?;

        Ok(AutoRecon {
            config: core_config,
            scan_context,
            runtime,
        })
    }

    /// Scan a single target
    ///
    /// # Arguments
    /// * `address` - Target IP address or hostname
    /// * `execute_callback` - JavaScript function to execute commands
    ///
    /// # Returns
    /// Promise that resolves with scan results
    ///
    /// # Example
    /// ```javascript
    /// const result = await autorecon.scanTarget('192.168.1.1', async (cmd) => {
    ///   const { stdout, stderr } = await exec(cmd);
    ///   return { stdout, stderr, exitCode: 0, durationMs: 100 };
    /// });
    /// ```
    #[napi]
    pub async fn scan_target(
        &self,
        address: String,
        execute_callback: JsFunction,
    ) -> Result<ScanResult> {
        let start = std::time::Instant::now();

        // Create executor with callback
        let executor = NodeExecutor::new(
            self.scan_context.concurrent_scans,
            execute_callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?,
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
            address.clone(),
            &self.scan_context.output_dir,
        );

        // Run scan
        let scan_result = scanner.scan_target(&mut target).await;

        let duration = start.elapsed();

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
                    address,
                    services,
                    base_dir: target.base_dir.to_string_lossy().to_string(),
                    duration_ms: duration.as_millis() as u32,
                    success: true,
                    error: None,
                })
            }
            Err(e) => Ok(ScanResult {
                address,
                services: vec![],
                base_dir: target.base_dir.to_string_lossy().to_string(),
                duration_ms: duration.as_millis() as u32,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List available scan profiles
    ///
    /// # Returns
    /// Array of profile names
    ///
    /// # Example
    /// ```javascript
    /// const profiles = autorecon.listProfiles();
    /// console.log(profiles); // ['default', 'quick', 'udp']
    /// ```
    #[napi]
    pub fn list_profiles(&self) -> Vec<String> {
        self.config
            .port_scan_profiles
            .keys()
            .cloned()
            .collect()
    }

    /// Get profile information
    ///
    /// # Arguments
    /// * `profile_name` - Name of the profile
    ///
    /// # Returns
    /// Profile information object or null if not found
    ///
    /// # Example
    /// ```javascript
    /// const info = autorecon.getProfileInfo('default');
    /// console.log(info.scans); // Array of scan names
    /// ```
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

    /// Check if a tool is available in the system
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool to check
    /// * `check_callback` - JavaScript function to check tool availability
    ///
    /// # Returns
    /// Promise that resolves to true if tool is available
    ///
    /// # Example
    /// ```javascript
    /// const hasNmap = await autorecon.checkTool('nmap', async (tool) => {
    ///   try {
    ///     await exec(`${tool} --version`);
    ///     return true;
    ///   } catch {
    ///     return false;
    ///   }
    /// });
    /// ```
    #[napi]
    pub async fn check_tool(
        &self,
        tool_name: String,
        check_callback: JsFunction,
    ) -> Result<bool> {
        let result: Result<bool> = check_callback
            .call(None, &[tool_name.into()])
            .map_err(|e| Error::from_reason(format!("Callback error: {}", e)))?;

        result
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
