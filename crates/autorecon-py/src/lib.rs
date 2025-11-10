//! Python bindings for AutoRecon using PyO3
//!
//! This module provides Python bindings for AutoRecon, enabling high-performance
//! network reconnaissance from Python.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;

mod executor;
mod output;

use executor::PyExecutor;
use output::PyOutputHandler;

/// Configuration for AutoRecon scanner
#[pyclass]
#[derive(Clone)]
struct AutoReconConfig {
    #[pyo3(get, set)]
    config_dir: String,
    #[pyo3(get, set)]
    profile: String,
    #[pyo3(get, set)]
    concurrent_targets: usize,
    #[pyo3(get, set)]
    concurrent_scans: usize,
    #[pyo3(get, set)]
    output_dir: String,
    #[pyo3(get, set)]
    verbosity: u8,
    #[pyo3(get, set)]
    heartbeat_interval: u64,
    #[pyo3(get, set)]
    nmap_extra: String,
}

#[pymethods]
impl AutoReconConfig {
    #[new]
    #[pyo3(signature = (config_dir="config".to_string(), profile="default".to_string(), concurrent_targets=5, concurrent_scans=10, output_dir="results".to_string(), verbosity=1, heartbeat_interval=60, nmap_extra="-vv --reason -Pn".to_string()))]
    fn new(
        config_dir: String,
        profile: String,
        concurrent_targets: usize,
        concurrent_scans: usize,
        output_dir: String,
        verbosity: u8,
        heartbeat_interval: u64,
        nmap_extra: String,
    ) -> Self {
        AutoReconConfig {
            config_dir,
            profile,
            concurrent_targets,
            concurrent_scans,
            output_dir,
            verbosity,
            heartbeat_interval,
            nmap_extra,
        }
    }
}

/// Detected service from port scanning
#[pyclass]
#[derive(Clone)]
struct DetectedService {
    #[pyo3(get)]
    protocol: String,
    #[pyo3(get)]
    port: u16,
    #[pyo3(get)]
    service: String,
    #[pyo3(get)]
    version: Option<String>,
}

/// Scan result for a target
#[pyclass]
struct ScanResult {
    #[pyo3(get)]
    address: String,
    #[pyo3(get)]
    services: Vec<DetectedService>,
    #[pyo3(get)]
    base_dir: String,
    #[pyo3(get)]
    duration_ms: u64,
    #[pyo3(get)]
    success: bool,
    #[pyo3(get)]
    error: Option<String>,
}

/// Main AutoRecon scanner class
#[pyclass]
struct AutoRecon {
    config: autorecon_core::config::Config,
    scan_context: autorecon_core::scanner::ScanContext,
}

#[pymethods]
impl AutoRecon {
    /// Create a new AutoRecon instance
    #[new]
    #[pyo3(signature = (config_dir="config".to_string(), profile="default".to_string(), output_dir="results".to_string(), concurrent_targets=5, concurrent_scans=10, verbosity=1, heartbeat_interval=60, nmap_extra="-vv --reason -Pn".to_string()))]
    fn new(
        config_dir: String,
        profile: String,
        output_dir: String,
        concurrent_targets: usize,
        concurrent_scans: usize,
        verbosity: u8,
        heartbeat_interval: u64,
        nmap_extra: String,
    ) -> PyResult<Self> {
        // Load configuration
        let core_config = autorecon_core::config::Config::load_from_dir(&PathBuf::from(&config_dir))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to load config: {}", e)))?;

        // Validate profile
        if !core_config.port_scan_profiles.contains_key(&profile) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Profile '{}' not found", profile),
            ));
        }

        // Create scan context
        let scan_context = autorecon_core::scanner::ScanContext {
            concurrent_targets,
            concurrent_scans,
            profile,
            output_dir: PathBuf::from(output_dir),
            verbosity,
            heartbeat_interval,
            nmap_extra,
        };

        Ok(AutoRecon {
            config: core_config,
            scan_context,
        })
    }

    /// Scan a single target
    fn scan_target<'py>(
        &self,
        py: Python<'py>,
        address: String,
        execute_callback: PyObject,
    ) -> PyResult<&'py PyAny> {
        let config = self.config.clone();
        let scan_context = self.scan_context.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let start = std::time::Instant::now();

            // Create executor with callback
            let executor = PyExecutor::new(
                scan_context.concurrent_scans,
                execute_callback,
            );

            // Create output handler
            let output_handler = PyOutputHandler::new(scan_context.verbosity);

            // Create scanner
            let scanner = autorecon_core::scanner::Scanner::new(
                config,
                executor,
                output_handler,
                scan_context.clone(),
            );

            // Create target
            let mut target = autorecon_core::scanner::Target::new(
                address.clone(),
                &scan_context.output_dir,
            );

            // Run scan
            let scan_result = scanner.scan_target(&mut target).await;

            let duration = start.elapsed();

            match scan_result {
                Ok(_) => {
                    // Convert services to Python format
                    let services = target
                        .services
                        .into_iter()
                        .map(|s| DetectedService {
                            protocol: s.protocol.as_str().to_string(),
                            port: s.port,
                            service: s.service,
                            version: s.version,
                        })
                        .collect();

                    Ok(ScanResult {
                        address,
                        services,
                        base_dir: target.base_dir.to_string_lossy().to_string(),
                        duration_ms: duration.as_millis() as u64,
                        success: true,
                        error: None,
                    })
                }
                Err(e) => Ok(ScanResult {
                    address,
                    services: vec![],
                    base_dir: target.base_dir.to_string_lossy().to_string(),
                    duration_ms: duration.as_millis() as u64,
                    success: false,
                    error: Some(e.to_string()),
                }),
            }
        })
    }

    /// List available scan profiles
    fn list_profiles(&self) -> Vec<String> {
        self.config
            .port_scan_profiles
            .keys()
            .cloned()
            .collect()
    }

    /// Get profile information
    fn get_profile_info(&self, py: Python, profile_name: String) -> Option<PyObject> {
        self.config
            .port_scan_profiles
            .get(&profile_name)
            .map(|profile| {
                let dict = PyDict::new(py);
                dict.set_item("name", profile_name).ok()?;
                dict.set_item("scan_count", profile.scans.len()).ok()?;
                let scans: Vec<String> = profile.scans.iter().map(|s| s.name.clone()).collect();
                dict.set_item("scans", scans).ok()?;
                Some(dict.into())
            })
            .flatten()
    }
}

/// AutoRecon Python module
#[pymodule]
fn autorecon(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AutoRecon>()?;
    m.add_class::<AutoReconConfig>()?;
    m.add_class::<DetectedService>()?;
    m.add_class::<ScanResult>()?;
    Ok(())
}
