//! Configuration parsing and management
//!
//! This module handles parsing TOML configuration files for:
//! - Port scan profiles
//! - Service scans
//! - Global patterns

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::patterns::Pattern;
use crate::{AutoReconError, Result};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Port scan profiles (e.g., "default", "quick", "udp")
    pub port_scan_profiles: HashMap<String, PortScanProfile>,

    /// Service-specific scans (e.g., "http", "ftp", "smb")
    pub service_scans: HashMap<String, ServiceScan>,

    /// Global patterns to match against all output
    pub global_patterns: Vec<Pattern>,

    /// Configuration variables (username_wordlist, password_wordlist, etc.)
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

impl Config {
    /// Load configuration from directory containing TOML files
    ///
    /// Expects:
    /// - port-scan-profiles.toml
    /// - service-scans.toml
    /// - global-patterns.toml
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let port_scan_profiles_path = dir.join("port-scan-profiles.toml");
        let service_scans_path = dir.join("service-scans.toml");
        let global_patterns_path = dir.join("global-patterns.toml");

        let port_scan_profiles = Self::load_port_scan_profiles(&port_scan_profiles_path)?;
        let (service_scans, variables) = Self::load_service_scans(&service_scans_path)?;
        let global_patterns = Self::load_global_patterns(&global_patterns_path)?;

        Ok(Config {
            port_scan_profiles,
            service_scans,
            global_patterns,
            variables,
        })
    }

    /// Load configuration from TOML strings (useful for WASM)
    pub fn load_from_strings(
        port_scan_profiles: &str,
        service_scans: &str,
        global_patterns: &str,
    ) -> Result<Self> {
        let port_scan_profiles = Self::parse_port_scan_profiles(port_scan_profiles)?;
        let (service_scans_map, variables) = Self::parse_service_scans(service_scans)?;
        let global_patterns = Self::parse_global_patterns(global_patterns)?;

        Ok(Config {
            port_scan_profiles,
            service_scans: service_scans_map,
            global_patterns,
            variables,
        })
    }

    fn load_port_scan_profiles(path: &Path) -> Result<HashMap<String, PortScanProfile>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AutoReconError::Io(format!("Failed to read {:?}: {}", path, e)))?;
        Self::parse_port_scan_profiles(&content)
    }

    fn parse_port_scan_profiles(content: &str) -> Result<HashMap<String, PortScanProfile>> {
        let parsed: HashMap<String, PortScanProfileToml> = toml::from_str(content)?;

        let mut profiles = HashMap::new();
        for (profile_name, profile_toml) in parsed {
            let mut scans = Vec::new();

            for (scan_name, scan_def) in profile_toml {
                let scan = PortScan {
                    name: scan_name,
                    port_scan: scan_def.port_scan,
                    service_detection: scan_def.service_detection,
                };
                scans.push(scan);
            }

            profiles.insert(profile_name, PortScanProfile { scans });
        }

        Ok(profiles)
    }

    fn load_service_scans(path: &Path) -> Result<(HashMap<String, ServiceScan>, HashMap<String, String>)> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AutoReconError::Io(format!("Failed to read {:?}: {}", path, e)))?;
        Self::parse_service_scans(&content)
    }

    fn parse_service_scans(content: &str) -> Result<(HashMap<String, ServiceScan>, HashMap<String, String>)> {
        let parsed: ServiceScansToml = toml::from_str(content)?;

        // Extract variables
        let mut variables = HashMap::new();
        if let Some(username_wordlist) = parsed.username_wordlist {
            variables.insert("username_wordlist".to_string(), username_wordlist);
        }
        if let Some(password_wordlist) = parsed.password_wordlist {
            variables.insert("password_wordlist".to_string(), password_wordlist);
        }

        // Parse service scans
        let mut service_scans = HashMap::new();
        for (service_name, service_def) in parsed.services {
            let scans = service_def.scan.into_iter().map(|s| ScanDefinition {
                name: s.name,
                command: s.command,
                run_once: s.run_once.unwrap_or(false),
                ports: s.ports,
                patterns: s.pattern.unwrap_or_default().into_iter().map(|p| Pattern {
                    description: p.description.unwrap_or_default(),
                    pattern: regex::Regex::new(&p.pattern).unwrap(),
                    is_critical: false,
                }).collect(),
            }).collect();

            let manual = service_def.manual.unwrap_or_default().into_iter().map(|m| ManualCommand {
                description: m.description.unwrap_or_default(),
                commands: m.commands,
            }).collect();

            service_scans.insert(
                service_name.clone(),
                ServiceScan {
                    name: service_name,
                    service_names: service_def.service_names,
                    ignore_service_names: service_def.ignore_service_names.unwrap_or_default(),
                    scans,
                    manual,
                },
            );
        }

        Ok((service_scans, variables))
    }

    fn load_global_patterns(path: &Path) -> Result<Vec<Pattern>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AutoReconError::Io(format!("Failed to read {:?}: {}", path, e)))?;
        Self::parse_global_patterns(&content)
    }

    fn parse_global_patterns(content: &str) -> Result<Vec<Pattern>> {
        let parsed: GlobalPatternsToml = toml::from_str(content)?;

        let patterns = parsed
            .pattern
            .into_iter()
            .map(|p| -> Result<Pattern> {
                Ok(Pattern {
                    description: p.description.unwrap_or_default(),
                    pattern: regex::Regex::new(&p.pattern).map_err(AutoReconError::from)?,
                    is_critical: false,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(patterns)
    }
}

/// Port scan profile (e.g., "default", "quick")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanProfile {
    pub scans: Vec<PortScan>,
}

/// Individual port scan within a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScan {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_scan: Option<ScanCommand>,
    pub service_detection: ScanCommand,
}

/// A scan command with pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCommand {
    pub command: String,
    pub pattern: String,
}

/// Service-specific scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceScan {
    pub name: String,
    pub service_names: Vec<String>,
    #[serde(default)]
    pub ignore_service_names: Vec<String>,
    pub scans: Vec<ScanDefinition>,
    #[serde(default)]
    pub manual: Vec<ManualCommand>,
}

impl ServiceScan {
    /// Check if this service scan matches the given service name
    pub fn matches_service(&self, service_name: &str) -> bool {
        // Check if service is in ignore list
        for ignore in &self.ignore_service_names {
            if let Ok(re) = regex::Regex::new(ignore) {
                if re.is_match(service_name) {
                    return false;
                }
            }
        }

        // Check if service matches any service name pattern
        for name_pattern in &self.service_names {
            if let Ok(re) = regex::Regex::new(name_pattern) {
                if re.is_match(service_name) {
                    return true;
                }
            }
        }

        false
    }
}

/// Individual scan definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDefinition {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub run_once: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<PortFilter>,
    #[serde(default)]
    pub patterns: Vec<Pattern>,
}

/// Port filter for scans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortFilter {
    #[serde(default)]
    pub tcp: Vec<u16>,
    #[serde(default)]
    pub udp: Vec<u16>,
}

/// Manual command suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCommand {
    pub description: String,
    pub commands: Vec<String>,
}

// TOML deserialization structures
type PortScanProfileToml = HashMap<String, PortScanDefinitionToml>;

#[derive(Debug, Deserialize)]
struct PortScanDefinitionToml {
    #[serde(rename = "port-scan")]
    port_scan: Option<ScanCommand>,
    #[serde(rename = "service-detection")]
    service_detection: ScanCommand,
}

#[derive(Debug, Deserialize)]
struct ServiceScansToml {
    #[serde(default)]
    username_wordlist: Option<String>,
    #[serde(default)]
    password_wordlist: Option<String>,
    #[serde(flatten)]
    services: HashMap<String, ServiceDefinitionToml>,
}

#[derive(Debug, Deserialize)]
struct ServiceDefinitionToml {
    #[serde(rename = "service-names")]
    service_names: Vec<String>,
    #[serde(rename = "ignore-service-names")]
    ignore_service_names: Option<Vec<String>>,
    scan: Vec<ScanDefinitionToml>,
    manual: Option<Vec<ManualCommandToml>>,
}

#[derive(Debug, Deserialize)]
struct ScanDefinitionToml {
    name: String,
    command: String,
    run_once: Option<bool>,
    ports: Option<PortFilter>,
    pattern: Option<Vec<PatternToml>>,
}

#[derive(Debug, Deserialize)]
struct ManualCommandToml {
    description: Option<String>,
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GlobalPatternsToml {
    pattern: Vec<PatternToml>,
}

#[derive(Debug, Deserialize)]
struct PatternToml {
    description: Option<String>,
    pattern: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_port_scan_profiles() {
        let toml = r#"
[default]
    [default.nmap-quick]
        [default.nmap-quick.service-detection]
        command = 'nmap -sV {address}'
        pattern = '^(?P<port>\d+)\/(?P<protocol>tcp).*open\s+(?P<service>\S+)'
"#;

        let profiles = Config::parse_port_scan_profiles(toml).unwrap();
        assert!(profiles.contains_key("default"));
        assert_eq!(profiles["default"].scans.len(), 1);
        assert_eq!(profiles["default"].scans[0].name, "nmap-quick");
    }

    #[test]
    fn test_service_matches() {
        let service_scan = ServiceScan {
            name: "http".to_string(),
            service_names: vec!["^http".to_string()],
            ignore_service_names: vec!["^nacn_http$".to_string()],
            scans: vec![],
            manual: vec![],
        };

        assert!(service_scan.matches_service("http"));
        assert!(service_scan.matches_service("https"));
        assert!(!service_scan.matches_service("nacn_http"));
        assert!(!service_scan.matches_service("ftp"));
    }

    #[test]
    fn test_parse_global_patterns() {
        let toml = r#"
[[pattern]]
description = 'Test pattern'
pattern = 'VULNERABLE'
"#;

        let patterns = Config::parse_global_patterns(toml).unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].description, "Test pattern");
    }
}
