//! Scanner orchestration and target management
//!
//! This module provides the main scanning logic that orchestrates port scans
//! and service enumeration.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;
use crate::executor::CommandExecutor;
use crate::output::{LogLevel, LogHandler, OutputHandler};
use crate::patterns::PatternMatcher;
use crate::services::{DetectedService, ServiceDetectionPattern};
use crate::{AutoReconError, Result};

/// Scanning context with configuration and settings
#[derive(Debug, Clone)]
pub struct ScanContext {
    /// Number of targets to scan concurrently
    pub concurrent_targets: usize,

    /// Number of scans to run per target concurrently
    pub concurrent_scans: usize,

    /// Port scan profile to use (e.g., "default", "quick")
    pub profile: String,

    /// Output directory for results
    pub output_dir: PathBuf,

    /// Verbosity level (0-2)
    pub verbosity: u8,

    /// Heartbeat interval in seconds (for progress updates)
    pub heartbeat_interval: u64,

    /// Additional nmap arguments
    pub nmap_extra: String,
}

impl Default for ScanContext {
    fn default() -> Self {
        ScanContext {
            concurrent_targets: 5,
            concurrent_scans: 10,
            profile: "default".to_string(),
            output_dir: PathBuf::from("results"),
            verbosity: 1,
            heartbeat_interval: 60,
            nmap_extra: "-vv --reason -Pn".to_string(),
        }
    }
}

/// Represents a scan target (IP address or hostname)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Target address (IP or hostname)
    pub address: String,

    /// Base directory for this target's results
    pub base_dir: PathBuf,

    /// Directory for scan output
    pub scan_dir: PathBuf,

    /// Directory for reports
    pub report_dir: PathBuf,

    /// Directory for exploit code
    pub exploit_dir: PathBuf,

    /// Directory for loot (extracted data)
    pub loot_dir: PathBuf,

    /// Detected services
    pub services: Vec<DetectedService>,
}

impl Target {
    /// Create a new target
    pub fn new(address: String, output_dir: &PathBuf) -> Self {
        let base_dir = output_dir.join(&address);
        let scan_dir = base_dir.join("scans");
        let report_dir = base_dir.join("report");
        let exploit_dir = base_dir.join("exploit");
        let loot_dir = base_dir.join("loot");

        Target {
            address,
            base_dir,
            scan_dir,
            report_dir,
            exploit_dir,
            loot_dir,
            services: Vec::new(),
        }
    }

    /// Create directory structure for this target
    pub async fn create_directories<H: OutputHandler>(&self, handler: &H) -> Result<()> {
        handler.create_directory(&self.scan_dir).await?;
        handler.create_directory(&self.scan_dir.join("xml")).await?;
        handler.create_directory(&self.report_dir).await?;
        handler.create_directory(&self.report_dir.join("screenshots")).await?;
        handler.create_directory(&self.exploit_dir).await?;
        handler.create_directory(&self.loot_dir).await?;

        // Create initial report files
        let notes_path = self.report_dir.join("notes.txt");
        let notes_template = format!(
            "# {} Notes\n\n## Services\n\n## Vulnerabilities\n\n## Exploits\n\n## Loot\n\n",
            self.address
        );
        handler.write_file(&notes_path, &notes_template).await?;

        Ok(())
    }

    /// Get path to commands log
    pub fn commands_log_path(&self) -> PathBuf {
        self.scan_dir.join("_commands.log")
    }

    /// Get path to errors log
    pub fn errors_log_path(&self) -> PathBuf {
        self.scan_dir.join("_errors.log")
    }

    /// Get path to patterns log
    pub fn patterns_log_path(&self) -> PathBuf {
        self.scan_dir.join("_patterns.log")
    }

    /// Get path to manual commands file
    pub fn manual_commands_path(&self) -> PathBuf {
        self.scan_dir.join("_manual_commands.txt")
    }
}

/// Main scanner that orchestrates scanning operations
pub struct Scanner<E: CommandExecutor, H: OutputHandler> {
    pub(crate) config: Arc<Config>,
    pub(crate) executor: Arc<E>,
    pub(crate) output_handler: Arc<H>,
    pub(crate) context: ScanContext,
}

// Implement Clone for Scanner
impl<E: CommandExecutor, H: OutputHandler> Clone for Scanner<E, H> {
    fn clone(&self) -> Self {
        Scanner {
            config: Arc::clone(&self.config),
            executor: Arc::clone(&self.executor),
            output_handler: Arc::clone(&self.output_handler),
            context: self.context.clone(),
        }
    }
}

impl<E: CommandExecutor, H: OutputHandler> Scanner<E, H> {
    /// Create a new scanner
    pub fn new(config: Config, executor: E, output_handler: H, context: ScanContext) -> Self {
        Scanner {
            config: Arc::new(config),
            executor: Arc::new(executor),
            output_handler: Arc::new(output_handler),
            context,
        }
    }

    /// Scan a single target
    pub async fn scan_target(&self, target: &mut Target) -> Result<()> {
        // Create directory structure
        target.create_directories(&*self.output_handler).await?;

        // Log start
        self.output_handler
            .log(
                &target.commands_log_path(),
                LogLevel::Info,
                &format!("Starting scan of target: {}", target.address),
            )
            .await?;

        // Get port scan profile
        let profile = self.config.port_scan_profiles.get(&self.context.profile)
            .ok_or_else(|| AutoReconError::Config(
                format!("Port scan profile '{}' not found", self.context.profile)
            ))?;

        // Run port scans and detect services
        for port_scan in &profile.scans {
            // Run port scan if defined
            if let Some(port_scan_cmd) = &port_scan.port_scan {
                self.run_port_scan(target, &port_scan.name, port_scan_cmd).await?;
            }

            // Run service detection
            let detected = self.run_service_detection(
                target,
                &port_scan.name,
                &port_scan.service_detection,
            ).await?;

            target.services.extend(detected);
        }

        // Deduplicate services
        let unique_services: Vec<_> = target.services
            .iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .cloned()
            .collect();
        target.services = unique_services;

        // Run service-specific scans
        self.run_service_scans(target).await?;

        // Log completion
        self.output_handler
            .log(
                &target.commands_log_path(),
                LogLevel::Info,
                &format!("Completed scan of target: {}", target.address),
            )
            .await?;

        Ok(())
    }

    /// Run a port scan command
    async fn run_port_scan(
        &self,
        target: &Target,
        scan_name: &str,
        scan_cmd: &crate::config::ScanCommand,
    ) -> Result<()> {
        let command = self.substitute_variables(&scan_cmd.command, target, None);

        // Log command
        self.output_handler
            .log(
                &target.commands_log_path(),
                LogLevel::Info,
                &format!("[{}] {}", scan_name, command),
            )
            .await?;

        // Execute command
        match self.executor.execute(&command).await {
            Ok(output) => {
                if !output.is_success() {
                    self.output_handler
                        .log(
                            &target.errors_log_path(),
                            LogLevel::Error,
                            &format!("[{}] Exit code: {}\n{}", scan_name, output.exit_code, output.stderr),
                        )
                        .await?;
                }
                Ok(())
            }
            Err(e) => {
                self.output_handler
                    .log(
                        &target.errors_log_path(),
                        LogLevel::Error,
                        &format!("[{}] {}", scan_name, e),
                    )
                    .await?;
                Err(e)
            }
        }
    }

    /// Run service detection and return detected services
    async fn run_service_detection(
        &self,
        target: &Target,
        scan_name: &str,
        scan_cmd: &crate::config::ScanCommand,
    ) -> Result<Vec<DetectedService>> {
        let command = self.substitute_variables(&scan_cmd.command, target, None);

        // Log command
        self.output_handler
            .log(
                &target.commands_log_path(),
                LogLevel::Info,
                &format!("[{}] {}", scan_name, command),
            )
            .await?;

        // Execute command
        let output = match self.executor.execute(&command).await {
            Ok(output) => output,
            Err(e) => {
                self.output_handler
                    .log(
                        &target.errors_log_path(),
                        LogLevel::Error,
                        &format!("[{}] {}", scan_name, e),
                    )
                    .await?;
                return Err(e);
            }
        };

        // Check for errors
        if !output.is_success() {
            self.output_handler
                .log(
                    &target.errors_log_path(),
                    LogLevel::Error,
                    &format!("[{}] Exit code: {}\n{}", scan_name, output.exit_code, output.stderr),
                )
                .await?;
        }

        // Parse services from output
        let pattern = ServiceDetectionPattern::new(&scan_cmd.pattern)?;
        let services = pattern.detect(&output.combined_output());

        // Check for pattern matches
        self.check_patterns(target, scan_name, &output.combined_output()).await?;

        Ok(services)
    }

    /// Run service-specific scans
    async fn run_service_scans(&self, target: &Target) -> Result<()> {
        let mut manual_commands = Vec::new();

        for service in &target.services {
            // Find matching service scans
            for (_, service_scan) in &self.config.service_scans {
                if service_scan.matches_service(&service.service) {
                    // Run scans
                    for scan_def in &service_scan.scans {
                        self.run_service_scan(target, service, scan_def).await?;
                    }

                    // Collect manual commands
                    for manual in &service_scan.manual {
                        for cmd_template in &manual.commands {
                            let cmd = self.substitute_variables(cmd_template, target, Some(service));
                            manual_commands.push(format!("# {}\n{}\n", manual.description, cmd));
                        }
                    }
                }
            }
        }

        // Write manual commands file
        if !manual_commands.is_empty() {
            let content = manual_commands.join("\n");
            self.output_handler
                .write_file(&target.manual_commands_path(), &content)
                .await?;
        }

        Ok(())
    }

    /// Run a single service-specific scan
    async fn run_service_scan(
        &self,
        target: &Target,
        service: &DetectedService,
        scan_def: &crate::config::ScanDefinition,
    ) -> Result<()> {
        let command = self.substitute_variables(&scan_def.command, target, Some(service));

        // Log command
        self.output_handler
            .log(
                &target.commands_log_path(),
                LogLevel::Info,
                &format!("[{}_{}] {}", service.service, scan_def.name, command),
            )
            .await?;

        // Execute command
        let output = match self.executor.execute(&command).await {
            Ok(output) => output,
            Err(e) => {
                self.output_handler
                    .log(
                        &target.errors_log_path(),
                        LogLevel::Error,
                        &format!("[{}_{}] {}", service.service, scan_def.name, e),
                    )
                    .await?;
                return Err(e);
            }
        };

        // Check for errors
        if !output.is_success() {
            self.output_handler
                .log(
                    &target.errors_log_path(),
                    LogLevel::Error,
                    &format!("[{}_{}] Exit code: {}\n{}", service.service, scan_def.name, output.exit_code, output.stderr),
                )
                .await?;
        }

        // Check for pattern matches
        self.check_patterns(target, &scan_def.name, &output.combined_output()).await?;

        Ok(())
    }

    /// Check output against patterns and log matches
    async fn check_patterns(&self, target: &Target, scan_name: &str, output: &str) -> Result<()> {
        let mut patterns = HashMap::new();
        for pattern in &self.config.global_patterns {
            patterns.entry("global".to_string()).or_insert_with(Vec::new).push(pattern.clone());
        }

        let matcher = PatternMatcher::new(self.config.global_patterns.clone(), patterns);
        let matches = matcher.match_patterns(output, Some(scan_name));

        for m in matches {
            self.output_handler
                .log(
                    &target.patterns_log_path(),
                    if m.is_critical { LogLevel::Error } else { LogLevel::Warn },
                    &format!("[{}] {}: {}", scan_name, m.description, m.matched_text),
                )
                .await?;
        }

        Ok(())
    }

    /// Substitute variables in command templates
    fn substitute_variables(
        &self,
        template: &str,
        target: &Target,
        service: Option<&DetectedService>,
    ) -> String {
        let mut result = template.to_string();

        // Basic variables
        result = result.replace("{address}", &target.address);
        result = result.replace("{scandir}", target.scan_dir.to_str().unwrap_or(""));
        result = result.replace("{nmap_extra}", &self.context.nmap_extra);

        // Service-specific variables
        if let Some(svc) = service {
            result = result.replace("{port}", &svc.port.to_string());
            result = result.replace("{protocol}", svc.protocol.as_str());
            result = result.replace("{service}", &svc.service);

            let scheme = if svc.service.contains("https") || svc.service.contains("ssl") {
                "https"
            } else {
                "http"
            };
            result = result.replace("{scheme}", scheme);

            let secure = if scheme == "https" { "True" } else { "False" };
            result = result.replace("{secure}", secure);
        }

        // Config variables
        for (key, value) in &self.config.variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }

        result
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_creation() {
        let target = Target::new("192.168.1.1".to_string(), &PathBuf::from("results"));
        assert_eq!(target.address, "192.168.1.1");
        assert!(target.scan_dir.ends_with("192.168.1.1/scans"));
        assert!(target.report_dir.ends_with("192.168.1.1/report"));
    }

    #[test]
    fn test_scan_context_default() {
        let ctx = ScanContext::default();
        assert_eq!(ctx.concurrent_targets, 5);
        assert_eq!(ctx.concurrent_scans, 10);
        assert_eq!(ctx.profile, "default");
    }
}
