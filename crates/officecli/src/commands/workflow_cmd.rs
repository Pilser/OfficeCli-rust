use clap::{Args, Subcommand};
use handler_common::{HandlerError, OutputFormat};
use std::collections::HashMap;

#[derive(Args)]
pub struct WorkflowCommand {
    #[command(subcommand)]
    pub action: WorkflowAction,
}

#[derive(Subcommand)]
pub enum WorkflowAction {
    /// Run a workflow by name
    Run {
        /// Workflow YAML file path
        name: String,
        /// Key=value parameters
        #[arg(long)]
        param: Option<Vec<String>>,
    },
    /// List available workflows (scans workflows/ directory)
    List,
    /// Validate a workflow YAML file
    Validate {
        /// Path to YAML file
        file: String,
    },
}

pub fn handle_workflow(cmd: WorkflowCommand, _format: OutputFormat) -> Result<String, HandlerError> {
    match cmd.action {
        WorkflowAction::Run { name, param } => {
            let yaml = std::fs::read_to_string(&name)
                .map_err(|e| HandlerError::OperationFailed(format!("read workflow: {}", e)))?;
            let workflow = workflow_runner::parse_workflow(&yaml)
                .map_err(|e| HandlerError::OperationFailed(e))?;

            let mut params = HashMap::new();
            if let Some(ref p) = param {
                for entry in p {
                    if let Some(eq) = entry.find('=') {
                        params.insert(entry[..eq].to_string(), entry[eq+1..].to_string());
                    }
                }
            }

            let results = workflow_runner::run_workflow(&workflow, &params)
                .map_err(|e| HandlerError::OperationFailed(e))?;

            let mut out = String::new();
            for r in &results {
                let status = if r.success { "OK" } else { "FAIL" };
                out.push_str(&format!("[{}] step {}: {}\n", status, r.step_index, r.command));
                if !r.output.is_empty() {
                    out.push_str(&format!("  output: {}\n", r.output.trim()));
                }
            }
            Ok(out)
        }
        WorkflowAction::List => {
            let dir = std::path::Path::new("workflows");
            if dir.exists() && dir.is_dir() {
                let mut names = Vec::new();
                for entry in std::fs::read_dir(dir).map_err(|e| HandlerError::OperationFailed(e.to_string()))? {
                    let entry = entry.map_err(|e| HandlerError::OperationFailed(e.to_string()))?;
                    if let Some(ext) = entry.path().extension() {
                        if ext == "yaml" || ext == "yml" {
                            if let Some(name) = entry.file_name().to_str() {
                                names.push(name.to_string());
                            }
                        }
                    }
                }
                names.sort();
                Ok(names.join("\n"))
            } else {
                Ok("No workflows directory found. Create a 'workflows/' directory with .yaml files.".to_string())
            }
        }
        WorkflowAction::Validate { file } => {
            let yaml = std::fs::read_to_string(&file)
                .map_err(|e| HandlerError::OperationFailed(format!("read file: {}", e)))?;
            match workflow_runner::parse_workflow(&yaml) {
                Ok(wf) => Ok(format!("Valid workflow: '{}' with {} step(s)", wf.name, wf.steps.len())),
                Err(e) => Err(HandlerError::OperationFailed(format!("Invalid workflow: {}", e))),
            }
        }
    }
}
