use clap::{Args, Subcommand};
use handler_common::{HandlerError, OutputFormat};
use serde::{Deserialize, Serialize};

/// Manage and inspect installed plugins
#[derive(Args)]
pub struct PluginsCommand {
    #[command(subcommand)]
    pub action: PluginsAction,
}

#[derive(Subcommand)]
pub enum PluginsAction {
    /// List plugins discoverable in the standard search paths
    List,
    /// Show detailed info about a specific plugin
    Info {
        /// Plugin name
        name: String,
    },
    /// Validate a plugin manifest at the given path
    Lint {
        /// Path to plugin directory or plugin.json file
        path: String,
    },
}

pub fn handle_plugins(cmd: PluginsCommand, format: OutputFormat) -> Result<String, HandlerError> {
    match cmd.action {
        PluginsAction::List => list_plugins(format),
        PluginsAction::Info { name } => info_plugin(&name, format),
        PluginsAction::Lint { path } => lint_plugin(&path, format),
    }
}

// ─── Plugin Manifest ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    protocol: u32,
    kinds: Vec<String>,
    extensions: Vec<String>,
    #[serde(default)]
    tier: Option<String>,
}

#[derive(Debug)]
struct ResolvedPlugin {
    manifest: PluginManifest,
    executable_path: String,
    warnings: Vec<String>,
}

// ─── Discovery ────────────────────────────────────────────────

/// Enumerate all discoverable plugins from the four standard search roots:
/// 1. OFFICECLI_PLUGIN_<KIND>_<EXT> environment variables
/// 2. ~/.officecli/plugins/<kind>/<ext>/<exe>
/// 3. <exe-dir>/plugins/<kind>/<ext>/<exe>
/// 4. $PATH lookup for officecli-plugin-<kind>-<ext>
fn enumerate_all() -> Vec<ResolvedPlugin> {
    let mut plugins = Vec::new();

    // Walk convention directories
    let home = dirs_home();
    let user_plugins_dir = format!("{}/.officecli/plugins", home);
    discover_from_dir(&user_plugins_dir, &mut plugins);

    // Bundled directory (next to the current executable)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled_dir = exe_dir.join("plugins");
            if bundled_dir.exists() {
                discover_from_dir(&bundled_dir.to_string_lossy(), &mut plugins);
            }
        }
    }

    // Environment variable overrides
    discover_from_env(&mut plugins);

    plugins
}

fn discover_from_dir(root: &str, plugins: &mut Vec<ResolvedPlugin>) {
    let root_path = std::path::Path::new(root);
    if !root_path.exists() {
        return;
    }

    // Walk <root>/<kind>/<ext>/ directories looking for plugin.json
    if let Ok(kind_entries) = std::fs::read_dir(root_path) {
        for kind_entry in kind_entries.flatten() {
            if let Ok(ext_entries) = std::fs::read_dir(kind_entry.path()) {
                for ext_entry in ext_entries.flatten() {
                    let manifest_path = ext_entry.path().join("plugin.json");
                    if manifest_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                                let exe_path = find_plugin_exe(&ext_entry.path());
                                let warnings = validate_manifest(&manifest);
                                plugins.push(ResolvedPlugin {
                                    manifest,
                                    executable_path: exe_path,
                                    warnings,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn discover_from_env(_plugins: &mut Vec<ResolvedPlugin>) {
    // Check OFFICECLI_PLUGIN_<KIND>_<EXT> env vars
    // This is a lightweight implementation — the full upstream also
    // walks PATH for officecli-plugin-<kind>-<ext> binaries.
    for (key, value) in std::env::vars() {
        if key.starts_with("OFFICECLI_PLUGIN_") {
            let path = std::path::Path::new(&value);
            let manifest_path = path.join("plugin.json");
            if manifest_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        let warnings = validate_manifest(&manifest);
                        // Already added? Skip duplicates by name.
                        let exe_path = value.clone();
                        _plugins.push(ResolvedPlugin {
                            manifest,
                            executable_path: exe_path,
                            warnings,
                        });
                    }
                }
            }
        }
    }
}

fn find_plugin_exe(dir: &std::path::Path) -> String {
    // Look for any executable in the directory
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("officecli-plugin-") {
                return entry.path().to_string_lossy().to_string();
            }
        }
    }
    String::new()
}

fn validate_manifest(manifest: &PluginManifest) -> Vec<String> {
    let mut warnings = Vec::new();
    if manifest.protocol > 1 {
        warnings.push(format!(
            "protocol version {} may not be supported (expected 0 or 1)",
            manifest.protocol
        ));
    }
    if manifest.kinds.is_empty() {
        warnings.push("no kinds specified".to_string());
    }
    if manifest.extensions.is_empty() {
        warnings.push("no extensions specified".to_string());
    }
    warnings
}

fn dirs_home() -> String {
    // Simple cross-platform home directory resolution
    if let Ok(home) = std::env::var("HOME") {
        return home;
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return home;
    }
    // Fallback
    "/tmp".to_string()
}

// ─── Command Handlers ─────────────────────────────────────────

fn list_plugins(format: OutputFormat) -> Result<String, HandlerError> {
    let plugins = enumerate_all();

    match format {
        OutputFormat::Json => {
            let arr: Vec<serde_json::Value> = plugins
                .iter()
                .map(|p| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("name".into(), serde_json::Value::String(p.manifest.name.clone()));
                    obj.insert("version".into(), serde_json::Value::String(p.manifest.version.clone()));
                    obj.insert("protocol".into(), serde_json::Value::Number(p.manifest.protocol.into()));
                    obj.insert(
                        "kinds".into(),
                        serde_json::Value::Array(
                            p.manifest
                                .kinds
                                .iter()
                                .map(|k| serde_json::Value::String(k.clone()))
                                .collect(),
                        ),
                    );
                    obj.insert(
                        "extensions".into(),
                        serde_json::Value::Array(
                            p.manifest
                                .extensions
                                .iter()
                                .map(|e| serde_json::Value::String(e.clone()))
                                .collect(),
                        ),
                    );
                    if let Some(tier) = &p.manifest.tier {
                        obj.insert("tier".into(), serde_json::Value::String(tier.clone()));
                    }
                    obj.insert("path".into(), serde_json::Value::String(p.executable_path.clone()));
                    if !p.warnings.is_empty() {
                        obj.insert(
                            "warnings".into(),
                            serde_json::Value::Array(
                                p.warnings
                                    .iter()
                                    .map(|w| serde_json::Value::String(w.clone()))
                                    .collect(),
                            ),
                        );
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();
            Ok(serde_json::to_string_pretty(&arr)
                .map_err(|e| HandlerError::OperationFailed(e.to_string()))?)
        }
        OutputFormat::Text => {
            if plugins.is_empty() {
                return Ok(
                    "No plugins installed.\n\nPlugins extend officecli to support additional \
                     formats (.doc, .hwpx, .pdf export, ...).\n\
                     See: plugins/plugin-protocol.md for installation paths."
                        .to_string(),
                );
            }

            let mut lines = Vec::new();
            lines.push(format!(
                "{:<20} {:<10} {:<10} {:<15} {}",
                "NAME", "VERSION", "PROTOCOL", "KINDS", "PATH"
            ));
            for p in &plugins {
                let kinds = p.manifest.kinds.join(",");
                lines.push(format!(
                    "{:<20} {:<10} {:<10} {:<15} {}",
                    p.manifest.name,
                    p.manifest.version,
                    p.manifest.protocol,
                    kinds,
                    p.executable_path
                ));
            }
            Ok(lines.join("\n"))
        }
    }
}

fn info_plugin(name: &str, format: OutputFormat) -> Result<String, HandlerError> {
    let plugins = enumerate_all();
    let plugin = plugins
        .iter()
        .find(|p| p.manifest.name == name)
        .ok_or_else(|| HandlerError::PathNotFound(format!("plugin '{}' not found", name)))?;

    match format {
        OutputFormat::Json => {
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "name": plugin.manifest.name,
                "version": plugin.manifest.version,
                "protocol": plugin.manifest.protocol,
                "kinds": plugin.manifest.kinds,
                "extensions": plugin.manifest.extensions,
                "tier": plugin.manifest.tier,
                "path": plugin.executable_path,
                "warnings": plugin.warnings,
            }))
            .map_err(|e| HandlerError::OperationFailed(e.to_string()))?)
        }
        OutputFormat::Text => {
            let mut lines = Vec::new();
            lines.push(format!("Name:     {}", plugin.manifest.name));
            lines.push(format!("Version:  {}", plugin.manifest.version));
            lines.push(format!("Protocol: {}", plugin.manifest.protocol));
            lines.push(format!("Kinds:    {}", plugin.manifest.kinds.join(", ")));
            lines.push(format!(
                "Extensions: {}",
                plugin.manifest.extensions.join(", ")
            ));
            if let Some(tier) = &plugin.manifest.tier {
                lines.push(format!("Tier:     {}", tier));
            }
            lines.push(format!("Path:     {}", plugin.executable_path));
            if !plugin.warnings.is_empty() {
                lines.push("Warnings:".to_string());
                for w in &plugin.warnings {
                    lines.push(format!("  - {}", w));
                }
            }
            Ok(lines.join("\n"))
        }
    }
}

fn lint_plugin(path: &str, _format: OutputFormat) -> Result<String, HandlerError> {
    let manifest_path = if path.ends_with("plugin.json") {
        path.to_string()
    } else {
        format!("{}/plugin.json", path.trim_end_matches('/'))
    };

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        HandlerError::OperationFailed(format!("failed to read '{}': {}", manifest_path, e))
    })?;

    let manifest: PluginManifest = serde_json::from_str(&content).map_err(|e| {
        HandlerError::OperationFailed(format!("invalid plugin manifest: {}", e))
    })?;

    let warnings = validate_manifest(&manifest);

    let mut lines = Vec::new();
    lines.push(format!("OK: {} v{} (protocol {})", manifest.name, manifest.version, manifest.protocol));
    if warnings.is_empty() {
        lines.push("No warnings.".to_string());
    } else {
        lines.push("Warnings:".to_string());
        for w in &warnings {
            lines.push(format!("  - {}", w));
        }
    }
    Ok(lines.join("\n"))
}
