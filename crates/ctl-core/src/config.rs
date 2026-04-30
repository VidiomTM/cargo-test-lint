use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub coverage: CoverageConfig,
    #[serde(default)]
    pub mutation: MutationConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_coverage_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_mutation_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default = "default_true")]
    pub filter_uncovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_full_sweep_interval")]
    pub full_sweep_interval_secs: u64,
    #[serde(default)]
    pub socket_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub show_diff_hunks: bool,
}

fn default_true() -> bool {
    true
}

fn default_coverage_timeout() -> u64 {
    300
}

fn default_mutation_timeout() -> u64 {
    600
}

fn default_debounce_ms() -> u64 {
    500
}

fn default_full_sweep_interval() -> u64 {
    300
}

fn default_level() -> String {
    "warning".to_owned()
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self { enabled: true, timeout_secs: 300, extra_args: Vec::new() }
    }
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self { enabled: true, timeout_secs: 600, extra_args: Vec::new(), filter_uncovered: true }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self { debounce_ms: 500, full_sweep_interval_secs: 300, socket_path: None }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self { level: "warning".to_owned(), show_diff_hunks: true }
    }
}

pub fn load(project_root: &Path) -> Config {
    match (try_load_ctl_toml(project_root), try_load_cargo_toml(project_root)) {
        (Some(c), _) => c,
        (None, Some(c)) => c,
        (None, None) => Config::default(),
    }
}

fn try_load_ctl_toml(project_root: &Path) -> Option<Config> {
    let path = project_root.join("ctl.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&content).ok()
}

fn try_load_cargo_toml(project_root: &Path) -> Option<Config> {
    let path = project_root.join("Cargo.toml");
    let content = std::fs::read_to_string(&path).ok()?;

    #[derive(Deserialize)]
    struct Manifest {
        package: Package,
    }

    #[derive(Deserialize)]
    struct Package {
        metadata: Option<Metadata>,
    }

    #[derive(Deserialize)]
    struct Metadata {
        ctl: Option<Config>,
    }

    let manifest: Manifest = toml::from_str(&content).ok()?;
    manifest.package.metadata?.ctl
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert!(config.coverage.enabled);
        assert_eq!(config.coverage.timeout_secs, 300);
        assert!(config.coverage.extra_args.is_empty());
        assert!(config.mutation.enabled);
        assert_eq!(config.mutation.timeout_secs, 600);
        assert!(config.mutation.filter_uncovered);
        assert_eq!(config.daemon.debounce_ms, 500);
        assert_eq!(config.daemon.full_sweep_interval_secs, 300);
        assert!(config.daemon.socket_path.is_none());
        assert_eq!(config.output.level, "warning");
        assert!(config.output.show_diff_hunks);
    }

    #[test]
    fn load_returns_defaults_when_no_files() {
        let tmp = tempfile::tempdir().unwrap();
        let config = load(tmp.path());
        assert_eq!(config.coverage.timeout_secs, 300);
    }

    #[test]
    fn ctl_toml_takes_precedence() {
        let tmp = tempfile::tempdir().unwrap();
        let ctl_path = tmp.path().join("ctl.toml");
        fs::write(&ctl_path, "[coverage]\ntimeout_secs = 99\n").unwrap();
        let cargo_path = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_path,
            "[package]\nname = \"x\"\n[package.metadata.ctl]\n[package.metadata.ctl.coverage]\ntimeout_secs = 42\n",
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.coverage.timeout_secs, 99);
    }

    #[test]
    fn cargo_toml_metadata_used_as_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let cargo_path = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_path,
            "[package]\nname = \"x\"\n[package.metadata.ctl]\n[package.metadata.ctl.mutation]\ntimeout_secs = 42\n",
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.mutation.timeout_secs, 42);
    }

    #[test]
    fn partial_override_keeps_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let ctl_path = tmp.path().join("ctl.toml");
        fs::write(&ctl_path, "[daemon]\ndebounce_ms = 100\n").unwrap();

        let config = load(tmp.path());
        assert_eq!(config.daemon.debounce_ms, 100);
        assert_eq!(config.daemon.full_sweep_interval_secs, 300);
        assert!(config.coverage.enabled);
    }
}
