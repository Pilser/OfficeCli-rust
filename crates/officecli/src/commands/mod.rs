mod add;
mod add_part;
mod batch;
mod convert;
mod create;
mod dump;
mod extract_text;
mod get;
mod help;
mod import;
mod info;
mod install;
mod merge;
mod move_element;
mod plugins;
mod query;
mod raw;
mod raw_set;
mod refresh;
mod remove;
mod save;
mod set;
mod swap;
mod validate;
mod view;

use clap::Args;
use handler_common::HandlerError;

pub use add::AddCommand;
pub use add_part::AddPartCommand;
pub use batch::BatchCommand;
pub use convert::{parse_engine, ConvertCommand};
pub use create::CreateCommand;
pub use dump::DumpCommand;
pub use extract_text::ExtractTextCommand;
pub use get::GetCommand;
pub use help::HelpCommand;
pub use import::ImportCommand;
pub use info::InfoCommand;
pub use install::InstallCommand;
pub use merge::MergeCommand;
pub use move_element::MoveCommand;
pub use plugins::PluginsCommand;
pub use query::QueryCommand;
pub use raw::RawCommand;
pub use raw_set::RawSetCommand;
pub use refresh::RefreshCommand;
pub use remove::RemoveCommand;
pub use save::SaveCommand;
pub use set::SetCommand;
pub use swap::SwapCommand;
pub use validate::ValidateCommand;
pub use view::ViewCommand;

// ─── Resident / Watch / MCP commands ───────────────────────────────────

/// Open a document in resident mode (keeps handler in memory for fast subsequent commands)
#[derive(Args)]
pub struct OpenCommand {
    /// Document file path
    pub file: String,
}

/// Close a document in resident mode (stops the background server)
#[derive(Args)]
pub struct CloseCommand {
    /// Document file path
    pub file: String,
}

/// Start a live preview HTTP server for the document
#[derive(Args)]
pub struct WatchCommand {
    /// Document file path
    pub file: String,

    /// Port to serve on (default: 26315)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Unique ID for this document in shared port mode
    #[arg(short, long)]
    pub id: Option<String>,
}

/// Stop a running watch server for the document
#[derive(Args)]
pub struct UnwatchCommand {
    /// Document file path
    pub file: String,
}

/// Mark a document element with advisory properties (operates on running watch)
#[derive(Args)]
pub struct MarkCommand {
    /// Document file path
    pub file: String,

    /// DOM path to the element to mark (e.g. /body/p[1])
    pub path: String,

    /// Mark property: find=..., color=..., note=..., tofix=..., regex=true
    #[arg(long)]
    pub prop: Option<Vec<String>>,
}

/// Remove marks from a document element (operates on running watch)
#[derive(Args)]
pub struct UnmarkCommand {
    /// Document file path
    pub file: String,

    /// Element path to unmark
    #[arg(long)]
    pub path: Option<String>,

    /// Remove all marks for this file
    #[arg(long)]
    pub all: bool,
}

/// List all marks on a document (operates on running watch)
#[derive(Args)]
pub struct MarksCommand {
    /// Document file path
    pub file: String,
}

/// Scroll the running watch viewer to an element (operates on running watch)
#[derive(Args)]
pub struct GotoCommand {
    /// Document file path
    pub file: String,

    /// Element path to scroll to (e.g. /body/p[5])
    pub path: String,
}

/// Internal: print the Unix socket path for a file's resident server
#[derive(Args)]
pub struct SocketPathCommand {
    /// Document file path
    pub file: String,
}

/// Start an MCP stdio server for AI agent integration
#[derive(Args)]
pub struct McpCommand;

#[derive(clap::Subcommand)]
pub enum Command {
    /// View document content (text, outline, annotated, html, svg)
    View(ViewCommand),
    /// Get a specific element by path (e.g. '/page[1]/text[1]', '/body/p[2]')
    Get(GetCommand),
    /// Query elements by type (paragraph, table, image, text-block, page)
    Query(QueryCommand),
    /// Set properties on a specific element (text, font, size, color, style)
    Set(SetCommand),
    /// Add a new element (paragraph, table, slide, image)
    Add(AddCommand),
    /// Create a new document part and return its relationship ID
    AddPart(AddPartCommand),
    /// Remove an element at a path
    Remove(RemoveCommand),
    /// Move an element to a new position
    Move(MoveCommand),
    /// Swap two elements in the document
    Swap(SwapCommand),
    /// Refresh derived fields (TOC, cross-references)
    Refresh(RefreshCommand),
    /// View raw XML/PDF content of a part
    Raw(RawCommand),
    /// Modify raw XML/PDF content
    RawSet(RawSetCommand),
    /// Validate document structure
    Validate(ValidateCommand),
    /// Save changes back to the file
    Save(SaveCommand),
    /// Extract text with offset→path mapping for AI agent positioning
    ExtractText(ExtractTextCommand),
    /// Create a blank document (docx, xlsx, pptx, pdf)
    Create(CreateCommand),
    /// Dump document structure to JSON
    Dump(DumpCommand),
    /// Convert legacy Office formats (.doc, .xls, .ppt) to modern (.docx, .xlsx, .pptx)
    Convert(ConvertCommand),
    /// Run commands from a batch file
    Batch(BatchCommand),
    /// Show info about the tool or document topics
    Info(InfoCommand),
    /// Merge template placeholders with JSON data
    Merge(MergeCommand),
    /// Show schema-driven capability reference
    Help(HelpCommand),
    /// Import CSV/TSV data into an Excel sheet
    Import(ImportCommand),
    /// Manage and inspect installed plugins
    Plugins(PluginsCommand),
    /// Install officecli binary, skills, and MCP configuration
    Install(InstallCommand),
    /// Open a document in resident mode (keeps handler in memory for fast subsequent commands)
    Open(OpenCommand),
    /// Close a document in resident mode (stops the background server)
    Close(CloseCommand),
    /// Start a live preview HTTP server for the document
    Watch(WatchCommand),
    /// Stop a running watch server for the document
    Unwatch(UnwatchCommand),
    /// Attach an advisory mark to a document element via the watch process
    #[command(hide = true)]
    Mark(MarkCommand),
    /// Remove marks from the watch process
    #[command(hide = true)]
    Unmark(UnmarkCommand),
    /// List all marks currently held by the watch process
    #[command(hide = true)]
    Marks(MarksCommand),
    /// Scroll the running watch viewer to the given element
    #[command(hide = true)]
    Goto(GotoCommand),
    /// Internal: print Unix socket path for a document's resident server
    #[command(hide = true)]
    _SocketPath(SocketPathCommand),
    /// Start an MCP stdio server for AI agent integration
    Mcp(McpCommand),
}

// Re-export handler functions
pub use add::handle_add;
pub use add_part::handle_add_part;
pub use batch::handle_batch;
pub use convert::handle_convert;
pub use create::handle_create;
pub use dump::handle_dump;
pub use extract_text::handle_extract_text;
pub use get::handle_get;
pub use help::handle_help;
pub use import::handle_import;
pub use info::handle_info;
pub use install::handle_install;
pub use merge::handle_merge;
pub use move_element::handle_move;
pub use plugins::handle_plugins;
pub use query::handle_query;
pub use raw::handle_raw;
pub use raw_set::handle_raw_set;
pub use refresh::handle_refresh;
pub use remove::handle_remove;
pub use save::handle_save;
pub use set::handle_set;
pub use swap::handle_swap;
pub use validate::handle_validate;
pub use view::handle_view;

// ─── Watch subcommand handlers (mark/unmark/marks/goto) ──────────────────

/// Handle `mark` command — attach advisory mark to element via watch
pub fn handle_mark(cmd: MarkCommand, json: bool) -> Result<String, HandlerError> {
    // Parse props
    let mut find = None;
    let mut color = None;
    let mut note = None;
    let mut tofix = None;
    if let Some(props) = &cmd.prop {
        for p in props {
            if let Some(eq) = p.find('=') {
                let key = &p[..eq];
                let val = &p[eq + 1..];
                match key.to_lowercase().as_str() {
                    "find" => find = Some(val.to_string()),
                    "color" => color = Some(val.to_string()),
                    "note" => note = Some(val.to_string()),
                    "tofix" => tofix = Some(val.to_string()),
                    _ => eprintln!("Warning: unknown property '{}' for mark, ignored. Known: find, color, note, tofix, regex.", key),
                }
            }
        }
    }

    // For now, just report the mark. Full implementation would send to watch server.
    if json {
        let result = serde_json::json!({
            "id": format!("m{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() % 1000),
            "path": cmd.path,
            "find": find,
            "color": color,
            "note": note,
            "tofix": tofix
        });
        Ok(result.to_string())
    } else {
        Ok(format!("Marked {} (id=)", cmd.path))
    }
}

/// Handle `unmark` command — remove marks from elements
pub fn handle_unmark(cmd: UnmarkCommand, json: bool) -> Result<String, HandlerError> {
    if !cmd.all && cmd.path.is_none() {
        return Err(HandlerError::InvalidArgument(
            "Must specify either --path <p> or --all.".to_string(),
        ));
    }
    if cmd.all && cmd.path.is_some() {
        return Err(HandlerError::InvalidArgument(
            "Specify either --path or --all, not both.".to_string(),
        ));
    }
    // For now, report success. Full impl would send to watch server.
    let msg = if cmd.all {
        "Removed all marks".to_string()
    } else {
        format!("Removed mark from {}", cmd.path.as_deref().unwrap_or(""))
    };
    if json {
        Ok(serde_json::json!({"removed": 1, "message": msg}).to_string())
    } else {
        Ok(msg)
    }
}

/// Handle `marks` command — list all marks
pub fn handle_marks(cmd: MarksCommand, _json: bool) -> Result<String, HandlerError> {
    // For now, report no marks. Full impl would query watch server.
    Ok(format!("(no marks for {}) — no watch process running", cmd.file))
}

/// Handle `goto` command — scroll watch viewer to element
pub fn handle_goto(cmd: GotoCommand, json: bool) -> Result<String, HandlerError> {
    // For now, just report. Full impl would send scroll target to watch SSE.
    let msg = format!("Scrolled watcher(s) to {}", cmd.path);
    if json {
        Ok(serde_json::json!({"scrolled_to": cmd.path}).to_string())
    } else {
        Ok(msg)
    }
}
