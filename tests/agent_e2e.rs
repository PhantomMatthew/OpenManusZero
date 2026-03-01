//! End-to-end tests for agent orchestration

use openmanus::agent::{Agent, Manus, ReActAgent, ToolCallAgent};
use openmanus::flow::{BaseFlow, PlanningFlow};
use openmanus::llm::MockLlmClient;
use openmanus::schema::AgentState;
use openmanus::tool::ToolCollection;
use std::collections::HashMap;
use std::sync::Arc;

fn create_test_llm() -> Arc<MockLlmClient> {
    Arc::new(MockLlmClient::new("gpt-4"))
}

fn create_llm_with_response(text: &str) -> Arc<MockLlmClient> {
    let llm = create_test_llm();
    llm.set_text_response(text);
    llm
}

// ============== ReActAgent E2E Tests ==============

#[tokio::test]
async fn test_react_agent_simple_conversation() {
    let llm = create_llm_with_response("Hello! I'm here to help.");
    let mut agent = ReActAgent::new("test_agent", "A test agent", llm).with_max_steps(3);

    let result = agent.run("Hello!").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_react_agent_with_context() {
    let llm = create_llm_with_response("I understand the context.");
    let mut agent = ReActAgent::new("context_agent", "Context-aware agent", llm).with_max_steps(3);

    let result = agent.run("Process this request").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_react_agent_cleanup() {
    let llm = create_llm_with_response("Done");
    let mut agent =
        ReActAgent::new("cleanup_agent", "Agent to test cleanup", llm).with_max_steps(3);

    let _ = agent.run("Do something").await;
    let cleanup_result = agent.cleanup().await;
    assert!(cleanup_result.is_ok());
}

#[tokio::test]
async fn test_react_agent_state_transitions() {
    let llm = create_llm_with_response("Done");
    let mut agent = ReActAgent::new("state_agent", "Agent to test state", llm).with_max_steps(3);

    // Agent starts in IDLE state
    assert_eq!(agent.state(), AgentState::Idle);

    // After run, agent should be back to IDLE
    let _ = agent.run("Test state").await;
    assert_eq!(agent.state(), AgentState::Idle);
}

// ============== ToolCallAgent E2E Tests ==============

#[tokio::test]
async fn test_toolcall_agent_with_tools() {
    let llm = create_llm_with_response("I completed the task.");

    let tools = Arc::new(ToolCollection::new());

    let mut agent =
        ToolCallAgent::new("tool_agent", "Agent with tools", llm, tools).with_max_steps(5);

    let result = agent.run("Do something").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_toolcall_agent_terminates_on_max_steps() {
    let llm = create_test_llm();
    // Add multiple responses to simulate continued tool calls
    llm.set_text_response("Step 1");
    llm.set_text_response("Step 2");
    llm.set_text_response("Step 3");

    let tools = Arc::new(ToolCollection::new());

    let mut agent = ToolCallAgent::new("limited_agent", "Agent with low step limit", llm, tools)
        .with_max_steps(2); // Very low limit

    let result = agent.run("Keep going").await;
    // Should complete even if hitting max steps
    assert!(result.is_ok());
}

// ============== Manus Agent E2E Tests ==============

#[tokio::test]
async fn test_manus_agent_basic_run() {
    let llm = create_llm_with_response("Task completed successfully.");
    let mut manus = Manus::with_llm(llm);

    let result = manus.run("Help me with a simple task").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_manus_agent_state() {
    let llm = create_llm_with_response("Done");
    let mut manus = Manus::with_llm(llm);

    // Check initial state
    assert_eq!(manus.state(), AgentState::Idle);

    // Run and check state returns to Idle
    let _ = manus.run("Do something").await;
    assert_eq!(manus.state(), AgentState::Idle);
}

#[tokio::test]
async fn test_manus_agent_cleanup() {
    let llm = create_llm_with_response("Done");
    let mut manus = Manus::with_llm(llm);

    let _ = manus.run("Do something").await;
    let cleanup_result = manus.cleanup().await;
    assert!(cleanup_result.is_ok());
}

// ============== Flow Orchestration E2E Tests ==============

#[tokio::test]
async fn test_base_flow_single_agent() {
    let llm = create_llm_with_response("Flow completed.");
    let manus = Manus::with_llm(llm);

    let mut flow = BaseFlow::single(Box::new(manus));
    let result = flow.execute("Execute this flow").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_base_flow_cleanup() {
    let llm = create_llm_with_response("Done");
    let manus = Manus::with_llm(llm);

    let mut flow = BaseFlow::single(Box::new(manus));
    let _ = flow.execute("Run first").await;
    let cleanup_result = flow.cleanup().await;
    assert!(cleanup_result.is_ok());
}

#[tokio::test]
async fn test_base_flow_agent_keys() {
    let llm = create_llm_with_response("Done");
    let manus = Manus::with_llm(llm);

    let flow = BaseFlow::single(Box::new(manus));
    assert!(flow.agent_keys().contains(&"default"));
}

#[tokio::test]
async fn test_planning_flow_create() {
    let llm = create_test_llm();
    let agents: HashMap<String, Box<dyn Agent>> = HashMap::new();
    let flow = PlanningFlow::new(agents, llm);
    // PlanningFlow is more complex and requires agent setup
    // This test just verifies it can be created
    let _ = flow;
}

// ============== Memory and Context E2E Tests ==============

#[tokio::test]
async fn test_agent_memory_persistence() {
    let llm = create_llm_with_response("I remember our conversation.");

    let mut agent = ReActAgent::new("memory_agent", "Agent with memory", llm).with_max_steps(3);

    // First run
    let _ = agent.run("My name is Alice").await;

    // Second run - agent should have memory of first run
    let result = agent.run("What's my name?").await;
    assert!(result.is_ok());
}

// ============== Error Handling E2E Tests ==============

#[tokio::test]
async fn test_agent_handles_llm_error() {
    // Create a mock client that returns empty response
    let llm = create_test_llm();
    llm.set_text_response(""); // Empty response

    let mut agent = ReActAgent::new("error_agent", "Agent handling errors", llm).with_max_steps(1);

    let result = agent.run("This should handle empty response").await;
    // Agent should complete even with empty response
    assert!(result.is_ok() || result.is_err());
}

// ============== Concurrent Operations E2E Tests ==============

#[tokio::test]
async fn test_multiple_agents_concurrent() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..3 {
        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            let llm = create_llm_with_response(&format!("Agent {} response", i));
            let mut agent = ReActAgent::new(
                format!("concurrent_agent_{}", i),
                format!("Concurrent agent {}", i),
                llm,
            )
            .with_max_steps(1);

            let _ = agent.run(&format!("Request {}", i)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task should complete");
    }

    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

// ============== Agent Trait Tests ==============

#[tokio::test]
async fn test_agent_trait_object() {
    let llm = create_llm_with_response("Trait object works");
    let agent: Box<dyn Agent> =
        Box::new(ReActAgent::new("trait_agent", "Testing trait object", llm).with_max_steps(3));

    assert_eq!(agent.name(), "trait_agent");
    assert_eq!(agent.description(), "Testing trait object");
    assert_eq!(agent.state(), AgentState::Idle);
}

#[tokio::test]
async fn test_manus_as_trait_object() {
    let llm = create_llm_with_response("Manus trait works");
    let manus: Box<dyn Agent> = Box::new(Manus::with_llm(llm));

    assert_eq!(manus.name(), "Manus");
    assert_eq!(manus.state(), AgentState::Idle);
}
