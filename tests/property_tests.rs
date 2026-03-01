//! Property-based tests using proptest

use openmanus::context::Memory;
use openmanus::llm::LlmRequest;
use openmanus::schema::{Message, Role, ToolCall};
use openmanus::tool::ToolCollection;
use proptest::prelude::*;

// ============== Message Property Tests ==============

proptest! {
    #[test]
    fn test_message_user_content_preserved(content in ".*") {
        let msg = Message::user(&content);
        prop_assert_eq!(msg.role, Role::User);
        prop_assert_eq!(msg.content, Some(content));
    }

    #[test]
    fn test_message_system_content_preserved(content in ".*") {
        let msg = Message::system(&content);
        prop_assert_eq!(msg.role, Role::System);
        prop_assert_eq!(msg.content, Some(content));
    }

    #[test]
    fn test_message_assistant_content_preserved(content in ".*") {
        let msg = Message::assistant(&content);
        prop_assert_eq!(msg.role, Role::Assistant);
        prop_assert_eq!(msg.content, Some(content));
    }

    #[test]
    fn test_message_serialization_roundtrip(
        role in prop_oneof![Just(Role::User), Just(Role::Assistant), Just(Role::System)],
        content in ".*"
    ) {
        let msg = Message {
            role,
            content: if content.is_empty() { None } else { Some(content.clone()) },
            tool_calls: None,
            name: None,
            tool_call_id: None,
            base64_image: None,
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(msg.role, deserialized.role);
        prop_assert_eq!(msg.content, deserialized.content);
    }
}

// ============== Memory Property Tests ==============

proptest! {
    #[test]
    fn test_memory_add_increases_count(messages in prop::collection::vec(".*", 0..100)) {
        let memory = Memory::new();

        for msg in &messages {
            memory.add(Message::user(msg));
        }

        prop_assert_eq!(memory.len(), messages.len());
    }

    #[test]
    fn test_memory_recent_returns_correct_count(
        total in 1usize..100,
        requested in 1usize..50
    ) {
        let memory = Memory::new();

        for i in 0..total {
            memory.add(Message::user(format!("Message {}", i)));
        }

        let recent = memory.recent(requested.min(total));
        let expected_len = requested.min(total);

        prop_assert_eq!(recent.len(), expected_len);
    }

    #[test]
    fn test_memory_clear_empties_all(count in 0usize..100) {
        let memory = Memory::new();

        for i in 0..count {
            memory.add(Message::user(format!("Message {}", i)));
        }

        memory.clear();
        prop_assert!(memory.is_empty());
        prop_assert_eq!(memory.len(), 0);
    }
}

// ============== ToolCollection Property Tests ==============

#[test]
fn test_tool_collection_empty_has_no_tools() {
    let collection = ToolCollection::new();
    assert!(collection.tool_names().is_empty());
}

// ============== LlmRequest Property Tests ==============

proptest! {
    #[test]
    fn test_llm_request_model_preserved(model in "[a-z0-9-]+") {
        let request = LlmRequest::new(&model, vec![]);
        prop_assert_eq!(request.model, model);
    }

    #[test]
    fn test_llm_request_messages_count(messages in prop::collection::vec(".*", 0..20)) {
        let msgs: Vec<Message> = messages.iter().map(Message::user).collect();
        let request = LlmRequest::new("gpt-4", msgs.clone());
        prop_assert_eq!(request.messages.len(), msgs.len());
    }

    #[test]
    fn test_llm_request_serialization_roundtrip(
        model in "[a-z0-9-]+",
        messages in prop::collection::vec(".*", 0..5)
    ) {
        let msgs: Vec<Message> = messages.iter().map(Message::user).collect();
        let request = LlmRequest::new(&model, msgs);

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: LlmRequest = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(request.model, deserialized.model);
        prop_assert_eq!(request.messages.len(), deserialized.messages.len());
    }
}

// ============== ToolCall Property Tests ==============

proptest! {
    #[test]
    fn test_tool_call_preserves_fields(
        id in "[a-zA-Z0-9_]+",
        name in "[a-z_]+",
        arguments in ".*"
    ) {
        let tool_call = ToolCall::new(&id, &name, &arguments);

        prop_assert_eq!(tool_call.id, id);
        prop_assert_eq!(tool_call.function.name, name);
        prop_assert_eq!(tool_call.function.arguments, arguments);
    }
}

// ============== Context Property Tests ==============

proptest! {
    #[test]
    fn test_context_env_roundtrip(key in "[A-Z_]+", value in ".*") {
        use openmanus::context::Context;

        let mut ctx = Context::new();
        ctx.set_env(&key, &value);

        prop_assert_eq!(ctx.get_env(&key), Some(&value));
    }
}
