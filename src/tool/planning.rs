//! Planning tool for creating and managing task plans

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Step status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    NotStarted,
    InProgress,
    Completed,
    Blocked,
}

impl Default for StepStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbol = match self {
            Self::NotStarted => "[ ]",
            Self::InProgress => "[→]",
            Self::Completed => "[✓]",
            Self::Blocked => "[!]",
        };
        write!(f, "{}", symbol)
    }
}

/// A single step in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step description
    pub description: String,
    /// Step status
    pub status: StepStatus,
    /// Additional notes
    pub notes: String,
}

/// A plan with multiple steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Unique plan identifier
    pub plan_id: String,
    /// Plan title
    pub title: String,
    /// Plan steps
    pub steps: Vec<PlanStep>,
}

impl Plan {
    /// Create a new plan
    pub fn new(plan_id: String, title: String, steps: Vec<String>) -> Self {
        Self {
            plan_id,
            title,
            steps: steps
                .into_iter()
                .map(|desc| PlanStep {
                    description: desc,
                    status: StepStatus::NotStarted,
                    notes: String::new(),
                })
                .collect(),
        }
    }

    /// Get progress statistics
    pub fn progress(&self) -> (usize, usize, usize, usize, usize) {
        let total = self.steps.len();
        let completed = self.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
        let in_progress = self.steps.iter().filter(|s| s.status == StepStatus::InProgress).count();
        let blocked = self.steps.iter().filter(|s| s.status == StepStatus::Blocked).count();
        let not_started = self.steps.iter().filter(|s| s.status == StepStatus::NotStarted).count();
        (total, completed, in_progress, blocked, not_started)
    }

    /// Format plan for display
    pub fn format(&self) -> String {
        let (total, completed, _, _, _) = self.progress();
        let percentage = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let mut output = format!("Plan: {} (ID: {})\n", self.title, self.plan_id);
        output.push_str(&"=".repeat(output.len().saturating_sub(1)));
        output.push_str("\n\n");
        output.push_str(&format!(
            "Progress: {}/{} steps completed ({:.1}%)\n\n",
            completed, total, percentage
        ));
        output.push_str("Steps:\n");

        for (i, step) in self.steps.iter().enumerate() {
            output.push_str(&format!("{}. {} {}\n", i, step.status, step.description));
            if !step.notes.is_empty() {
                output.push_str(&format!("   Notes: {}\n", step.notes));
            }
        }

        output
    }
}

/// Planning tool command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PlanningCommand {
    Create {
        plan_id: String,
        title: String,
        steps: Vec<String>,
    },
    Update {
        plan_id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        steps: Option<Vec<String>>,
    },
    List,
    Get {
        #[serde(default)]
        plan_id: Option<String>,
    },
    SetActive {
        plan_id: String,
    },
    MarkStep {
        #[serde(default)]
        plan_id: Option<String>,
        step_index: usize,
        #[serde(default)]
        step_status: Option<StepStatus>,
        #[serde(default)]
        step_notes: Option<String>,
    },
    Delete {
        plan_id: String,
    },
}

/// Planning tool for managing task plans
pub struct PlanningTool {
    /// Stored plans
    plans: Arc<RwLock<HashMap<String, Plan>>>,
    /// Current active plan ID
    active_plan_id: Arc<RwLock<Option<String>>>,
}

impl PlanningTool {
    /// Create a new planning tool
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
            active_plan_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Execute a planning command
    pub async fn execute_command(&self, command: PlanningCommand) -> Result<String, ToolError> {
        match command {
            PlanningCommand::Create { plan_id, title, steps } => {
                self.create_plan(plan_id, title, steps).await
            }
            PlanningCommand::Update { plan_id, title, steps } => {
                self.update_plan(plan_id, title, steps).await
            }
            PlanningCommand::List => self.list_plans().await,
            PlanningCommand::Get { plan_id } => self.get_plan(plan_id).await,
            PlanningCommand::SetActive { plan_id } => self.set_active_plan(plan_id).await,
            PlanningCommand::MarkStep { plan_id, step_index, step_status, step_notes } => {
                self.mark_step(plan_id, step_index, step_status, step_notes).await
            }
            PlanningCommand::Delete { plan_id } => self.delete_plan(plan_id).await,
        }
    }

    async fn create_plan(&self, plan_id: String, title: String, steps: Vec<String>) -> Result<String, ToolError> {
        let mut plans = self.plans.write().await;

        if plans.contains_key(&plan_id) {
            return Err(ToolError::ExecutionFailed(format!(
                "A plan with ID '{}' already exists",
                plan_id
            )));
        }

        if steps.is_empty() {
            return Err(ToolError::InvalidInput(
                "Steps must be a non-empty list".to_string(),
            ));
        }

        let plan = Plan::new(plan_id.clone(), title, steps);
        let output = format!("Plan created successfully with ID: {}\n\n{}", plan_id, plan.format());

        plans.insert(plan_id.clone(), plan);

        // Set as active plan
        let mut active = self.active_plan_id.write().await;
        *active = Some(plan_id);

        Ok(output)
    }

    async fn update_plan(
        &self,
        plan_id: String,
        title: Option<String>,
        steps: Option<Vec<String>>,
    ) -> Result<String, ToolError> {
        let mut plans = self.plans.write().await;

        let plan = plans.get_mut(&plan_id).ok_or_else(|| {
            ToolError::NotFound(format!("No plan found with ID: {}", plan_id))
        })?;

        if let Some(new_title) = title {
            plan.title = new_title;
        }

        if let Some(new_steps) = steps {
            // Preserve status for unchanged steps
            let old_steps: Vec<_> = plan.steps.iter().map(|s| s.description.clone()).collect();
            let old_statuses: Vec<_> = plan.steps.iter().map(|s| s.status.clone()).collect();
            let old_notes: Vec<_> = plan.steps.iter().map(|s| s.notes.clone()).collect();

            plan.steps = new_steps
                .into_iter()
                .enumerate()
                .map(|(i, desc)| {
                    if i < old_steps.len() && desc == old_steps[i] {
                        PlanStep {
                            description: desc,
                            status: old_statuses[i].clone(),
                            notes: old_notes[i].clone(),
                        }
                    } else {
                        PlanStep {
                            description: desc,
                            status: StepStatus::NotStarted,
                            notes: String::new(),
                        }
                    }
                })
                .collect();
        }

        Ok(format!(
            "Plan updated successfully: {}\n\n{}",
            plan_id,
            plan.format()
        ))
    }

    async fn list_plans(&self) -> Result<String, ToolError> {
        let plans = self.plans.read().await;
        let active = self.active_plan_id.read().await;

        if plans.is_empty() {
            return Ok("No plans available. Create a plan with the 'create' command.".to_string());
        }

        let mut output = "Available plans:\n".to_string();

        for (plan_id, plan) in plans.iter() {
            let (total, completed, _, _, _) = plan.progress();
            let marker = if active.as_ref() == Some(plan_id) { " (active)" } else { "" };
            output.push_str(&format!(
                "• {}{}: {} - {}/{} steps completed\n",
                plan_id, marker, plan.title, completed, total
            ));
        }

        Ok(output)
    }

    async fn get_plan(&self, plan_id: Option<String>) -> Result<String, ToolError> {
        let plans = self.plans.read().await;
        let active = self.active_plan_id.read().await;

        let id = plan_id.or_else(|| active.clone()).ok_or_else(|| {
            ToolError::ExecutionFailed(
                "No active plan. Please specify a plan_id or set an active plan.".to_string(),
            )
        })?;

        let plan = plans.get(&id).ok_or_else(|| {
            ToolError::NotFound(format!("No plan found with ID: {}", id))
        })?;

        Ok(plan.format())
    }

    async fn set_active_plan(&self, plan_id: String) -> Result<String, ToolError> {
        let plans = self.plans.read().await;

        let plan = plans.get(&plan_id).ok_or_else(|| {
            ToolError::NotFound(format!("No plan found with ID: {}", plan_id))
        })?;

        let mut active = self.active_plan_id.write().await;
        *active = Some(plan_id.clone());

        Ok(format!(
            "Plan '{}' is now the active plan.\n\n{}",
            plan_id,
            plan.format()
        ))
    }

    async fn mark_step(
        &self,
        plan_id: Option<String>,
        step_index: usize,
        step_status: Option<StepStatus>,
        step_notes: Option<String>,
    ) -> Result<String, ToolError> {
        let active = self.active_plan_id.read().await;
        let id = plan_id.or_else(|| active.clone()).ok_or_else(|| {
            ToolError::ExecutionFailed(
                "No active plan. Please specify a plan_id or set an active plan.".to_string(),
            )
        })?;
        drop(active);

        let mut plans = self.plans.write().await;

        let plan = plans.get_mut(&id).ok_or_else(|| {
            ToolError::NotFound(format!("No plan found with ID: {}", id))
        })?;

        if step_index >= plan.steps.len() {
            return Err(ToolError::InvalidInput(format!(
                "Invalid step_index: {}. Valid indices range from 0 to {}",
                step_index,
                plan.steps.len().saturating_sub(1)
            )));
        }

        if let Some(status) = step_status {
            plan.steps[step_index].status = status;
        }

        if let Some(notes) = step_notes {
            plan.steps[step_index].notes = notes;
        }

        Ok(format!(
            "Step {} updated in plan '{}'.\n\n{}",
            step_index,
            id,
            plan.format()
        ))
    }

    async fn delete_plan(&self, plan_id: String) -> Result<String, ToolError> {
        let mut plans = self.plans.write().await;

        if plans.remove(&plan_id).is_none() {
            return Err(ToolError::NotFound(format!(
                "No plan found with ID: {}",
                plan_id
            )));
        }

        // Clear active plan if it was the deleted one
        let mut active = self.active_plan_id.write().await;
        if active.as_ref() == Some(&plan_id) {
            *active = None;
        }

        Ok(format!("Plan '{}' has been deleted.", plan_id))
    }
}

impl Default for PlanningTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PlanningTool {
    fn name(&self) -> &'static str {
        "planning"
    }

    fn description(&self) -> &'static str {
        "A planning tool that allows the agent to create and manage plans for solving complex tasks"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "command".to_string(),
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: Some("The command to execute: create, update, list, get, set_active, mark_step, delete".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "create".to_string(),
                            "update".to_string(),
                            "list".to_string(),
                            "get".to_string(),
                            "set_active".to_string(),
                            "mark_step".to_string(),
                            "delete".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "plan_id".to_string(),
                    ToolParameter {
                        name: "plan_id".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Unique identifier for the plan".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "title".to_string(),
                    ToolParameter {
                        name: "title".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Title for the plan".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "steps".to_string(),
                    ToolParameter {
                        name: "steps".to_string(),
                        param_type: "array".to_string(),
                        description: Some("List of plan steps".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "step_index".to_string(),
                    ToolParameter {
                        name: "step_index".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Index of the step to update (0-based)".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "step_status".to_string(),
                    ToolParameter {
                        name: "step_status".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Status to set for a step: not_started, in_progress, completed, blocked".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: Some(vec![
                            "not_started".to_string(),
                            "in_progress".to_string(),
                            "completed".to_string(),
                            "blocked".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "step_notes".to_string(),
                    ToolParameter {
                        name: "step_notes".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Additional notes for a step".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["command".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let command: PlanningCommand = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid command: {}", e)))?;

        let output = self.execute_command(command).await?;
        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_plan() {
        let tool = PlanningTool::new();
        let result = tool
            .execute_command(PlanningCommand::Create {
                plan_id: "test-plan".to_string(),
                title: "Test Plan".to_string(),
                steps: vec!["Step 1".to_string(), "Step 2".to_string()],
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Plan created successfully"));
    }

    #[tokio::test]
    async fn test_list_plans() {
        let tool = PlanningTool::new();

        // Create a plan first
        tool.execute_command(PlanningCommand::Create {
            plan_id: "plan-1".to_string(),
            title: "Plan 1".to_string(),
            steps: vec!["Step A".to_string()],
        })
        .await
        .unwrap();

        let result = tool.execute_command(PlanningCommand::List).await.unwrap();
        assert!(result.contains("plan-1"));
    }

    #[tokio::test]
    async fn test_mark_step() {
        let tool = PlanningTool::new();

        tool.execute_command(PlanningCommand::Create {
            plan_id: "test".to_string(),
            title: "Test".to_string(),
            steps: vec!["Step 1".to_string(), "Step 2".to_string()],
        })
        .await
        .unwrap();

        let result = tool
            .execute_command(PlanningCommand::MarkStep {
                plan_id: Some("test".to_string()),
                step_index: 0,
                step_status: Some(StepStatus::Completed),
                step_notes: Some("Done!".to_string()),
            })
            .await
            .unwrap();

        assert!(result.contains("[✓]"));
    }

    #[tokio::test]
    async fn test_delete_plan() {
        let tool = PlanningTool::new();

        tool.execute_command(PlanningCommand::Create {
            plan_id: "to-delete".to_string(),
            title: "Delete Me".to_string(),
            steps: vec!["Step".to_string()],
        })
        .await
        .unwrap();

        let result = tool
            .execute_command(PlanningCommand::Delete {
                plan_id: "to-delete".to_string(),
            })
            .await
            .unwrap();

        assert!(result.contains("deleted"));
    }

    #[test]
    fn test_plan_format() {
        let plan = Plan::new(
            "test".to_string(),
            "Test Plan".to_string(),
            vec!["Step 1".to_string(), "Step 2".to_string()],
        );

        let formatted = plan.format();
        assert!(formatted.contains("Test Plan"));
        assert!(formatted.contains("0/2"));
    }

    #[test]
    fn test_step_status_display() {
        assert_eq!(format!("{}", StepStatus::NotStarted), "[ ]");
        assert_eq!(format!("{}", StepStatus::InProgress), "[→]");
        assert_eq!(format!("{}", StepStatus::Completed), "[✓]");
        assert_eq!(format!("{}", StepStatus::Blocked), "[!]");
    }
}
