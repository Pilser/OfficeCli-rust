use clap::Args;
use handler_common::HandlerError;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertEngine {
    LibreOffice,
    Oxide,
}

impl std::str::FromStr for ConvertEngine {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "libreoffice" | "lo" => Ok(Self::LibreOffice),
            "oxide" => Ok(Self::Oxide),
            other => Err(format!(
                "unknown engine '{}' (choose: libreoffice, oxide)",
                other
            )),
        }
    }
}

/// Helper for MCP to parse engine string without needing FromStr in scope.
pub fn parse_engine(s: &str) -> Result<ConvertEngine, String> {
    s.parse()
}

/// Convert a legacy Office document (.doc, .xls, .ppt) to modern format (.docx, .xlsx, .pptx)
#[derive(Args)]
#[command(after_help = "\
SUPPORTED CONVERSIONS:
  .doc  -> .docx   Word legacy binary to modern OOXML
  .xls  -> .xlsx   Excel legacy binary to modern OOXML
  .ppt  -> .pptx   PowerPoint legacy binary to modern OOXML
  .pdf  -> .docx   PDF to Word (LibreOffice only)
  .docx -> .docx   Re-save / normalize modern Word (same for .xlsx, .pptx)

Cross-family conversions other than PDF->DOCX are NOT supported.

CONVERSION ENGINES:
  libreoffice (default)                    oxide
  ─────────────────────                    ─────
  High fidelity (~1:1)                     Lower fidelity (via IR, may lose styles/headers/objects)
  Needs LibreOffice installed (~700MB)     Pure Rust, no external dependency
  Slower (process spawn overhead)          Fast (sub-second)
  Preserves formatting, images, tables     Preserves basic content and structure
  Supports PDF -> DOCX                     Same-family only (no PDF support)

  Install LibreOffice:
    macOS:  brew install --cask libreoffice
    Ubuntu: sudo apt install libreoffice
    Windows: https://www.libreoffice.org/download/

EXAMPLES:
  officecli convert old.doc                       Convert .doc -> .docx via LibreOffice (default)
  officecli convert old.xls -o report.xlsx        Convert with custom output name
  officecli convert old.ppt --force               Convert, overwrite existing output
  officecli convert input.pdf -o output.docx      Convert PDF to Word (requires LibreOffice)
  officecli convert old.doc --engine oxide        Convert via oxide (no LibreOffice needed)")]
pub struct ConvertCommand {
    /// Input file path (.doc, .xls, .ppt, .docx, .xlsx, .pptx, .pdf)
    pub file: String,

    /// Output file path (defaults to input path with updated extension)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Overwrite output file if it already exists
    #[arg(long)]
    pub force: bool,

    /// Conversion engine: libreoffice (default, high fidelity) or oxide (pure Rust, fast)
    #[arg(long, default_value = "libreoffice")]
    pub engine: ConvertEngine,
}

pub fn handle_convert(
    cmd: ConvertCommand,
    format: handler_common::OutputFormat,
) -> Result<String, HandlerError> {
    let input_path = PathBuf::from(&cmd.file);
    let input_ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // Determine target extension based on input family
    let target_ext = match input_ext.as_str() {
        "doc" | "docx" => "docx",
        "xls" | "xlsx" => "xlsx",
        "ppt" | "pptx" => "pptx",
        "pdf" => "docx",
        other => {
            return Err(HandlerError::UnsupportedMode(format!(
                "convert from '.{}' not supported (supported: .doc, .xls, .ppt, .docx, .xlsx, .pptx, .pdf)",
                other
            )));
        }
    };

    // Check input file exists
    if !input_path.exists() {
        return Err(HandlerError::OpenError(format!(
            "input file '{}' not found",
            cmd.file
        )));
    }

    // Derive output path
    let output_path = cmd
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension(target_ext));

    // Validate output extension matches input family
    let output_ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    validate_conversion(&input_ext, &output_ext, cmd.engine)?;

    // Prevent accidental overwrite
    if output_path.exists() && !cmd.force {
        return Err(HandlerError::OperationFailed(format!(
            "output file '{}' already exists; use --force to overwrite",
            output_path.display()
        )));
    }

    // If the target is a foreign extension that the built-in native handlers
    // don't speak, delegate to a plugin exporter before falling back to the
    // built-in engines (LibreOffice / oxide). This makes
    // `officecli convert foo.docx bar.html` route through whatever
    // exporter plugin is installed for `.html`.
    if !is_native_office_ext(&output_ext)
        && !is_native_office_ext(&input_ext)
    {
        // Both foreign: the user wants, say, .doc → .html. We can do this
        // in two hops (dump-reader to native, then exporter to target) if
        // both plugins are installed.
        if let Some(dump_result) = try_plugin_dump(&input_ext, &cmd.file)? {
            if let Some(export_result) = try_plugin_export(
                dump_result.target_family(),
                &output_ext,
                &dump_result.converted_path,
                &output_path,
            )? {
                // Best-effort cleanup of the intermediate native sibling.
                let _ = std::fs::remove_file(&dump_result.converted_path);
                return match format {
                    handler_common::OutputFormat::Text => Ok(format!(
                        "Converted '{}' -> '{}' via dump-reader '{}' then exporter '{}'",
                        cmd.file,
                        export_result.output_path,
                        dump_result.plugin_name,
                        export_result.plugin_name
                    )),
                    handler_common::OutputFormat::Json => Ok(serde_json::json!({
                        "input": cmd.file,
                        "output": export_result.output_path,
                        "from_format": input_ext,
                        "to_format": output_ext,
                        "engine": "plugin-pipeline",
                        "stages": [dump_result.plugin_name, export_result.plugin_name],
                    })
                    .to_string()),
                };
            }
            // No exporter for native→foreign here — fall back to error below.
            let _ = std::fs::remove_file(&dump_result.converted_path);
        }
        return Err(HandlerError::UnsupportedMode(format!(
            "cannot convert .{} → .{} via plugin (need dump-reader + exporter). \
             See plugins/plugin-protocol.md.",
            input_ext, output_ext
        )));
    }
    if !is_native_office_ext(&output_ext)
        && is_native_office_ext(&input_ext)
    {
        if let Some(export_result) = try_plugin_export(&input_ext, &output_ext, &cmd.file, &output_path)? {
            return match format {
                handler_common::OutputFormat::Text => Ok(format!(
                    "Converted '{}' -> '{}' via plugin '{}'",
                    cmd.file, export_result.output_path, export_result.plugin_name
                )),
                handler_common::OutputFormat::Json => Ok(serde_json::json!({
                    "input": cmd.file,
                    "output": export_result.output_path,
                    "from_format": input_ext,
                    "to_format": output_ext,
                    "engine": "plugin",
                    "plugin": export_result.plugin_name,
                })
                .to_string()),
            };
        }
        // No plugin — fall through to existing engines below.
    }

    // Foreign source → native target: try a dump-reader plugin first.
    // A `.doc → .docx` invocation should route to the installed dump-reader
    // before falling back to LibreOffice, since dump-readers preserve more
    // structure than LibreOffice's generic conversion.
    if !is_native_office_ext(&input_ext)
        && is_native_office_ext(&output_ext)
        && output_ext == family_from_foreign(&input_ext)
    {
        if let Some(dump_result) = try_plugin_dump(&input_ext, &cmd.file)? {
            // The dump-reader wrote to its own sibling path; if that's not
            // the same as `output_path`, copy across (or rename).
            if dump_result.converted_path != output_path.to_string_lossy() {
                std::fs::rename(&dump_result.converted_path, &output_path)
                    .or_else(|_| {
                        std::fs::copy(&dump_result.converted_path, &output_path)
                            .map(|_| ())
                    })
                    .map_err(|e| {
                        HandlerError::OperationFailed(format!(
                            "failed to move dump-reader output to '{}': {}",
                            output_path.display(),
                            e
                        ))
                    })?;
            }
            return match format {
                handler_common::OutputFormat::Text => Ok(format!(
                    "Converted '{}' -> '{}' via dump-reader plugin '{}' ({} items)",
                    cmd.file,
                    output_path.display(),
                    dump_result.plugin_name,
                    dump_result.items_replayed
                )),
                handler_common::OutputFormat::Json => Ok(serde_json::json!({
                    "input": cmd.file,
                    "output": output_path.to_string_lossy(),
                    "from_format": input_ext,
                    "to_format": output_ext,
                    "engine": "plugin-dump-reader",
                    "plugin": dump_result.plugin_name,
                    "items_replayed": dump_result.items_replayed,
                })
                .to_string()),
            };
        }
        // No dump-reader installed — fall through to LibreOffice.
    }

    // Perform conversion with selected engine
    match cmd.engine {
        ConvertEngine::LibreOffice => convert_via_libreoffice(&cmd.file, &output_path, target_ext)?,
        ConvertEngine::Oxide => convert_via_oxide(&cmd.file, &output_path)?,
    }

    match format {
        handler_common::OutputFormat::Text => Ok(format!(
            "Converted '{}' -> '{}' [{}]",
            cmd.file,
            output_path.display(),
            match cmd.engine {
                ConvertEngine::LibreOffice => "libreoffice",
                ConvertEngine::Oxide => "oxide",
            }
        )),
        handler_common::OutputFormat::Json => Ok(serde_json::json!({
            "input": cmd.file,
            "output": output_path.to_string_lossy(),
            "from_format": input_ext,
            "to_format": target_ext,
            "engine": match cmd.engine {
                ConvertEngine::LibreOffice => "libreoffice",
                ConvertEngine::Oxide => "oxide",
            },
        })
        .to_string()),
    }
}

/// Convert via LibreOffice CLI (soffice --convert-to).
fn convert_via_libreoffice(
    input_file: &str,
    output_path: &std::path::Path,
    target_ext: &str,
) -> Result<(), HandlerError> {
    let soffice = find_soffice()?;

    // soffice --convert-to writes to --outdir, using the input filename stem + target ext
    // We need to handle the case where --output specifies a different filename
    let output_dir = match output_path.parent() {
        Some(p) if p.as_os_str().is_empty() => std::path::Path::new("."),
        Some(p) => p,
        None => std::path::Path::new("."),
    };

    // Ensure output directory exists
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).map_err(|e| {
            HandlerError::OperationFailed(format!("cannot create output dir: {}", e))
        })?;
    }

    let output_dir_str = output_dir.to_string_lossy().to_string();

    // Determine input extension for PDF-specific filter
    let input_ext = std::path::Path::new(input_file)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let mut cmd = std::process::Command::new(&soffice);
    cmd.arg("--headless");

    // PDF requires writer_pdf_import filter, otherwise soffice silently fails
    if input_ext == "pdf" {
        cmd.arg("--infilter=writer_pdf_import");
    }

    cmd.arg("--convert-to")
        .arg(target_ext)
        .arg("--outdir")
        .arg(&output_dir_str)
        .arg(input_file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let status = cmd
        .status()
        .map_err(|e| HandlerError::OperationFailed(format!("failed to run soffice: {}", e)))?;

    if !status.success() {
        return Err(HandlerError::OperationFailed(format!(
            "soffice conversion failed (exit code {})",
            status.code().unwrap_or(-1)
        )));
    }

    // soffice generates output as <input_stem>.<target_ext> in output_dir
    let input_path_for_stem = PathBuf::from(input_file);
    let input_stem = input_path_for_stem
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let soffice_output = output_dir.join(format!("{}.{}", input_stem, target_ext));

    // If soffice output differs from desired output path, rename it
    if soffice_output != *output_path && soffice_output.exists() {
        std::fs::rename(&soffice_output, output_path).map_err(|e| {
            HandlerError::OperationFailed(format!(
                "failed to rename '{}' to '{}': {}",
                soffice_output.display(),
                output_path.display(),
                e
            ))
        })?;
    }

    // Verify the output file was created
    if !output_path.exists() {
        return Err(HandlerError::OperationFailed(format!(
            "soffice did not produce output file '{}'",
            output_path.display()
        )));
    }

    Ok(())
}

/// Find soffice executable, with helpful install hints if missing.
fn find_soffice() -> Result<String, HandlerError> {
    let candidates = [
        "soffice",
        "/usr/bin/soffice",
        "/usr/local/bin/soffice",
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
    ];

    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return Ok(candidate.to_string());
        }
    }

    // Try `which` on Unix platforms
    #[cfg(unix)]
    {
        let which_output = std::process::Command::new("which")
            .arg("soffice")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();
        if let Ok(output) = which_output {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                return Ok(path);
            }
        }
    }

    Err(HandlerError::OperationFailed(
        "LibreOffice (soffice) not found.\n\nInstall it:\n  macOS:  brew install --cask libreoffice\n  Ubuntu: sudo apt install libreoffice\n  Windows: https://www.libreoffice.org/download/\n\nOr use --engine oxide for pure Rust conversion (lower fidelity)".to_string(),
    ))
}

/// Convert via office_oxide (pure Rust, lower fidelity).
fn convert_via_oxide(input_file: &str, output_path: &std::path::Path) -> Result<(), HandlerError> {
    let doc = office_oxide::Document::open(input_file)
        .map_err(|e| HandlerError::OpenError(format!("failed to open '{}': {}", input_file, e)))?;

    doc.save_as(output_path.to_str().unwrap_or_default())
        .map_err(|e| HandlerError::SaveError(format!("failed to convert: {}", e)))?;

    Ok(())
}

/// Whether `ext` is a native Office extension the in-tree handlers speak.
fn is_native_office_ext(ext: &str) -> bool {
    matches!(ext, "docx" | "xlsx" | "pptx")
}

/// Map a legacy/foreign extension to its native sibling.
fn family_from_foreign(ext: &str) -> &'static str {
    match ext {
        "doc" => "docx",
        "xls" => "xlsx",
        "ppt" => "pptx",
        _ => "docx",
    }
}

/// Try to delegate the conversion to an installed exporter plugin. Returns
/// `Ok(None)` when no plugin is registered for `(input_ext, output_ext)`;
/// the caller falls back to the built-in engines.
fn try_plugin_export(
    _input_ext: &str,
    output_ext: &str,
    input_file: &str,
    output_path: &std::path::Path,
) -> Result<Option<super::plugin_process::ExportResult>, HandlerError> {
    match super::plugin_process::run_exporter(
        input_file,
        output_ext,
        &output_path.to_string_lossy(),
    ) {
        Ok(res) => Ok(Some(res)),
        Err(HandlerError::UnsupportedMode(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Try to delegate foreign-source migration to a dump-reader plugin. Writes
/// the native sibling file into a temporary path next to `input_file`. Returns
/// `Ok(None)` when no plugin is registered for `input_ext`.
fn try_plugin_dump(
    input_ext: &str,
    input_file: &str,
) -> Result<Option<super::plugin_process::DumpResult>, HandlerError> {
    if super::plugin_process::resolve_dump_reader(input_ext).is_none() {
        return Ok(None);
    }
    // Derive a sibling output path. The dump-reader's manifest `target`
    // selects the family; we don't know it without consulting the manifest,
    // so we guess from `input_ext` (`.doc` → `.docx`, `.xls` → `.xlsx`,
    // `.ppt` → `.pptx`). If wrong, the plugin's manifest target will
    // already have steered the skeleton creation, so the on-disk file is
    // correct — only our guess at the sibling name is wrong.
    let input_path = PathBuf::from(input_file);
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted")
        .to_string();
    let dir = input_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let family_guess = match input_ext {
        "doc" => "docx",
        "xls" => "xlsx",
        "ppt" => "pptx",
        _ => "docx",
    };
    let sibling = dir.join(format!("{}.{}", stem, family_guess));
    match super::plugin_process::run_dump_reader(input_file, &sibling.to_string_lossy()) {
        Ok(r) => Ok(Some(r)),
        Err(HandlerError::UnsupportedMode(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Validate that the conversion is supported.
fn validate_conversion(
    input_ext: &str,
    output_ext: &str,
    engine: ConvertEngine,
) -> Result<(), HandlerError> {
    // Cross-family: PDF -> DOCX (LibreOffice only)
    // Foreign target extensions are valid if a plugin exporter exists.
    // Skip the family-rule check below — those only cover native→native.
    if !is_native_office_ext(output_ext) {
        if super::plugin_process::resolve_exporter(input_ext, output_ext).is_some() {
            return Ok(());
        }
        // No direct exporter for (input_ext, output_ext) — but a
        // foreign→foreign pipeline through a dump-reader + exporter may
        // still succeed. Allow validate to pass; the dispatcher in
        // handle_convert handles the case where the pipeline is missing.
        if !is_native_office_ext(input_ext)
            && super::plugin_process::resolve_dump_reader(input_ext).is_some()
        {
            return Ok(());
        }
        return Err(HandlerError::UnsupportedMode(format!(
            "no plugin exporter handles .{} → .{} — install one or see plugins/plugin-protocol.md",
            input_ext, output_ext
        )));
    }

    if input_ext == "pdf" && output_ext == "docx" {
        if engine != ConvertEngine::LibreOffice {
            return Err(HandlerError::UnsupportedMode(
                "PDF to DOCX conversion requires LibreOffice engine (--engine libreoffice)"
                    .to_string(),
            ));
        }
        return Ok(());
    }
    if input_ext == "pdf" && output_ext != "docx" {
        return Err(HandlerError::UnsupportedMode(format!(
            "cannot convert .pdf to .{}; .pdf files can only convert to .docx",
            output_ext
        )));
    }

    // Same-family conversions
    let families = [
        (&["doc", "docx"][..], "docx"),
        (&["xls", "xlsx"][..], "xlsx"),
        (&["ppt", "pptx"][..], "pptx"),
    ];

    for (members, target) in families {
        if members.contains(&input_ext) {
            if output_ext != target {
                return Err(HandlerError::UnsupportedMode(format!(
                    "cannot convert .{} to .{}; .{} files must convert to .{}",
                    input_ext, output_ext, input_ext, target
                )));
            }
            return Ok(());
        }
    }

    Err(HandlerError::UnsupportedMode(format!(
        "unsupported input format: .{}",
        input_ext
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_valid_doc_to_docx() {
        assert!(validate_conversion("doc", "docx", ConvertEngine::LibreOffice).is_ok());
    }

    #[test]
    fn test_valid_xls_to_xlsx() {
        assert!(validate_conversion("xls", "xlsx", ConvertEngine::LibreOffice).is_ok());
    }

    #[test]
    fn test_valid_ppt_to_pptx() {
        assert!(validate_conversion("ppt", "pptx", ConvertEngine::LibreOffice).is_ok());
    }

    #[test]
    fn test_valid_docx_resave() {
        assert!(validate_conversion("docx", "docx", ConvertEngine::Oxide).is_ok());
    }

    #[test]
    fn test_valid_pdf_to_docx() {
        assert!(validate_conversion("pdf", "docx", ConvertEngine::LibreOffice).is_ok());
    }

    #[test]
    fn test_pdf_to_docx_requires_libreoffice() {
        let result = validate_conversion("pdf", "docx", ConvertEngine::Oxide);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cross_family() {
        let result = validate_conversion("doc", "xlsx", ConvertEngine::LibreOffice);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HandlerError::UnsupportedMode(_)));
    }

    #[test]
    fn test_unsupported_input_format() {
        let result = validate_conversion("odt", "docx", ConvertEngine::LibreOffice);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_from_str() {
        assert_eq!(
            ConvertEngine::from_str("libreoffice").unwrap(),
            ConvertEngine::LibreOffice
        );
        assert_eq!(
            ConvertEngine::from_str("lo").unwrap(),
            ConvertEngine::LibreOffice
        );
        assert_eq!(
            ConvertEngine::from_str("oxide").unwrap(),
            ConvertEngine::Oxide
        );
        assert!(ConvertEngine::from_str("foo").is_err());
    }
}
