use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub params: Vec<WorkflowParam>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub command: String,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index: usize,
    pub command: String,
    pub success: bool,
    pub output: String,
}

/// Parse a YAML workflow definition string.
pub fn parse_workflow(yaml: &str) -> Result<Workflow, String> {
    serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))
}

/// Run a workflow, substituting {{param}} placeholders in commands.
/// Returns results for each step.
pub fn run_workflow(
    workflow: &Workflow,
    params: &HashMap<String, String>,
) -> Result<Vec<StepResult>, String> {
    let mut results = Vec::new();

    for (i, step) in workflow.steps.iter().enumerate() {
        let mut command = step.command.clone();
        for (key, val) in params {
            command = command.replace(&format!("{{{{{}}}}}", key), val);
        }

        let should_run = match step.condition.as_deref() {
            None | Some("always") => true,
            Some("on-success") => results.last().map(|r: &StepResult| r.success).unwrap_or(true),
            Some("on-error") => results.last().map(|r| !r.success).unwrap_or(false),
            Some(other) => return Err(format!("Unknown condition: {}", other)),
        };

        if !should_run {
            results.push(StepResult {
                step_index: i,
                command: command.clone(),
                success: true,
                output: "(skipped by condition)".to_string(),
            });
            continue;
        }

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| format!("exec error: {}", e))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let full_output = if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };

        results.push(StepResult {
            step_index: i,
            command: command.clone(),
            success,
            output: full_output,
        });

        if !success && step.condition.as_deref() != Some("always") {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let yaml = r#"
name: test-workflow
description: A test
steps:
  - command: echo hello
"#;
        let wf = parse_workflow(yaml).unwrap();
        assert_eq!(wf.name, "test-workflow");
        assert_eq!(wf.steps.len(), 1);
    }

    #[test]
    fn test_parse_with_params() {
        let yaml = r#"
name: param-workflow
params:
  - name: name
    type: string
    required: true
steps:
  - command: echo {{name}}
"#;
        let wf = parse_workflow(yaml).unwrap();
        assert_eq!(wf.params.len(), 1);
        assert_eq!(wf.params[0].name, "name");
        assert!(wf.params[0].required);
    }

    #[test]
    fn test_run_substitution() {
        let yaml = r#"
name: sub-test
steps:
  - command: echo {{greeting}}
"#;
        let wf = parse_workflow(yaml).unwrap();
        let mut params = HashMap::new();
        params.insert("greeting".to_string(), "hello".to_string());
        let results = run_workflow(&wf, &params).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_condition_skip() {
        let yaml = r#"
name: cond-test
steps:
  - command: "false"
  - command: echo skipped
    condition: on-success
"#;
        let wf = parse_workflow(yaml).unwrap();
        let params = HashMap::new();
        let results = run_workflow(&wf, &params).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_invalid_yaml() {
        let result = parse_workflow("not: valid: yaml: [[[");
        assert!(result.is_err());
    }
}
