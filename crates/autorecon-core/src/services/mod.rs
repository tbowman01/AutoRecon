//! Service detection from scan output
//!
//! This module provides functionality for detecting services from port scan output.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

/// Network protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

impl std::str::FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Protocol::Tcp),
            "udp" => Ok(Protocol::Udp),
            _ => Err(format!("Unknown protocol: {}", s)),
        }
    }
}

/// A detected service from port scanning
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DetectedService {
    /// The protocol (tcp/udp)
    pub protocol: Protocol,

    /// The port number
    pub port: u16,

    /// The service name (e.g., "http", "ssh", "smb")
    pub service: String,

    /// Optional version information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info: Option<String>,
}

impl DetectedService {
    /// Create a new detected service
    pub fn new(protocol: Protocol, port: u16, service: String) -> Self {
        DetectedService {
            protocol,
            port,
            service,
            version: None,
            extra_info: None,
        }
    }

    /// Create with version info
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Create with extra info
    pub fn with_extra_info(mut self, info: String) -> Self {
        self.extra_info = Some(info);
        self
    }

    /// Get a unique key for this service (protocol:port)
    pub fn key(&self) -> String {
        format!("{}:{}", self.protocol.as_str(), self.port)
    }
}

/// Service detection pattern from configuration
#[derive(Debug, Clone)]
pub struct ServiceDetectionPattern {
    /// Regex pattern with named groups: port, protocol, service
    pub pattern: Regex,

    /// Optional version extraction pattern
    pub version_pattern: Option<Regex>,
}

impl ServiceDetectionPattern {
    /// Create a new service detection pattern
    ///
    /// The pattern must have named capture groups: (?P<port>...), (?P<protocol>...), (?P<service>...)
    pub fn new(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;

        // Verify required capture groups exist
        let capture_names: Vec<_> = regex.capture_names().collect();
        let has_port = capture_names.iter().any(|n| *n == Some("port"));
        let has_protocol = capture_names.iter().any(|n| *n == Some("protocol"));
        let has_service = capture_names.iter().any(|n| *n == Some("service"));

        if !has_port || !has_protocol || !has_service {
            return Err(crate::AutoReconError::Config(
                "Service detection pattern must have 'port', 'protocol', and 'service' named groups"
                    .to_string(),
            ));
        }

        Ok(ServiceDetectionPattern {
            pattern: regex,
            version_pattern: None,
        })
    }

    /// Set the version extraction pattern
    pub fn with_version_pattern(mut self, pattern: &str) -> Result<Self> {
        self.version_pattern = Some(Regex::new(pattern)?);
        Ok(self)
    }

    /// Detect services from text
    pub fn detect(&self, text: &str) -> Vec<DetectedService> {
        let mut services = Vec::new();

        for cap in self.pattern.captures_iter(text) {
            if let (Some(port_str), Some(protocol_str), Some(service_str)) = (
                cap.name("port"),
                cap.name("protocol"),
                cap.name("service"),
            ) {
                // Parse port
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    // Parse protocol
                    if let Ok(protocol) = protocol_str.as_str().parse::<Protocol>() {
                        let service = service_str.as_str().to_string();

                        // Try to extract version if pattern is provided
                        let version = cap
                            .name("version")
                            .map(|v| v.as_str().to_string());

                        let mut detected = DetectedService::new(protocol, port, service);
                        if let Some(v) = version {
                            detected = detected.with_version(v);
                        }

                        services.push(detected);
                    }
                }
            }
        }

        services
    }
}

/// Service detector that manages multiple detection patterns
#[derive(Debug)]
pub struct ServiceDetector {
    patterns: Vec<ServiceDetectionPattern>,
}

impl ServiceDetector {
    /// Create a new service detector
    pub fn new() -> Self {
        ServiceDetector {
            patterns: Vec::new(),
        }
    }

    /// Add a detection pattern
    pub fn add_pattern(&mut self, pattern: ServiceDetectionPattern) {
        self.patterns.push(pattern);
    }

    /// Detect all services from text
    ///
    /// Returns a deduplicated set of detected services
    pub fn detect(&self, text: &str) -> Vec<DetectedService> {
        let mut services = HashMap::new();

        for pattern in &self.patterns {
            for service in pattern.detect(text) {
                // Use the service key to deduplicate
                services.entry(service.key()).or_insert(service);
            }
        }

        services.into_values().collect()
    }
}

impl Default for ServiceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_parsing() {
        assert_eq!("tcp".parse::<Protocol>().unwrap(), Protocol::Tcp);
        assert_eq!("TCP".parse::<Protocol>().unwrap(), Protocol::Tcp);
        assert_eq!("udp".parse::<Protocol>().unwrap(), Protocol::Udp);
        assert!("invalid".parse::<Protocol>().is_err());
    }

    #[test]
    fn test_detected_service_key() {
        let service = DetectedService::new(Protocol::Tcp, 80, "http".to_string());
        assert_eq!(service.key(), "tcp:80");
    }

    #[test]
    fn test_service_detection_pattern() {
        let pattern = ServiceDetectionPattern::new(
            r"(?P<port>\d+)/(?P<protocol>tcp|udp)\s+open\s+(?P<service>\S+)",
        )
        .unwrap();

        let text = "80/tcp open http\n443/tcp open https\n53/udp open domain";
        let services = pattern.detect(text);

        assert_eq!(services.len(), 3);
        assert_eq!(services[0].port, 80);
        assert_eq!(services[0].protocol, Protocol::Tcp);
        assert_eq!(services[0].service, "http");
        assert_eq!(services[1].port, 443);
        assert_eq!(services[2].protocol, Protocol::Udp);
    }

    #[test]
    fn test_service_detector_deduplication() {
        let pattern1 = ServiceDetectionPattern::new(
            r"(?P<port>\d+)/(?P<protocol>tcp)\s+(?P<service>\S+)",
        )
        .unwrap();

        let pattern2 = ServiceDetectionPattern::new(
            r"Port (?P<port>\d+) \((?P<protocol>tcp)\): (?P<service>\S+)",
        )
        .unwrap();

        let mut detector = ServiceDetector::new();
        detector.add_pattern(pattern1);
        detector.add_pattern(pattern2);

        // Both patterns should detect the same service, but it should be deduplicated
        let text = "80/tcp http\nPort 80 (tcp): http";
        let services = detector.detect(text);

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].port, 80);
    }

    #[test]
    fn test_invalid_pattern() {
        // Missing required named groups
        let result = ServiceDetectionPattern::new(r"(\d+)");
        assert!(result.is_err());
    }
}
