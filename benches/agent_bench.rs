//! Benchmarks for OpenManus agent operations

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

fn bench_message_creation(c: &mut Criterion) {
    use openmanus::schema::Message;

    c.bench_function("message_user", |b| {
        b.iter(|| Message::user(black_box("Hello, world!")))
    });

    c.bench_function("message_system", |b| {
        b.iter(|| Message::system(black_box("You are a helpful assistant")))
    });

    c.bench_function("message_assistant", |b| {
        b.iter(|| Message::assistant(black_box("I can help with that.")))
    });

    c.bench_function("message_user_with_image", |b| {
        b.iter(|| {
            Message::user_with_image(
                black_box("What's in this image?"),
                black_box("base64imagedata"),
            )
        })
    });
}

fn bench_memory_operations(c: &mut Criterion) {
    use openmanus::context::Memory;
    use openmanus::schema::Message;

    c.bench_function("memory_add", |b| {
        let memory = Memory::new();
        b.iter(|| {
            memory.add(Message::user(black_box("test message")));
        })
    });

    c.bench_function("memory_recent", |b| {
        let memory = Memory::new();
        for i in 0..100 {
            memory.add(Message::user(format!("Message {}", i)));
        }
        b.iter(|| memory.recent(black_box(10)))
    });

    c.bench_function("memory_clear", |b| {
        b.iter_batched(
            || {
                let memory = Memory::new();
                for i in 0..50 {
                    memory.add(Message::user(format!("Message {}", i)));
                }
                memory
            },
            |memory| memory.clear(),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("memory_large_add", |b| {
        let memory = Memory::new();
        let large_text = "x".repeat(1000);
        b.iter(|| {
            memory.add(Message::user(black_box(large_text.clone())));
        })
    });
}

fn bench_tool_collection(c: &mut Criterion) {
    use openmanus::tool::ask_human::AskHumanTool;
    use openmanus::tool::bash::BashTool;
    use openmanus::tool::terminate::TerminateTool;
    use openmanus::tool::Tool;
    use openmanus::tool::ToolCollection;
    use std::sync::Arc;

    c.bench_function("tool_collection_new", |b| b.iter(ToolCollection::new));

    c.bench_function("tool_collection_add_tool", |b| {
        b.iter(|| {
            let mut collection = ToolCollection::new();
            collection.add_tool(Arc::new(BashTool::new()) as Arc<dyn Tool>);
            collection
        })
    });

    c.bench_function("tool_collection_to_definitions", |b| {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(BashTool::new()) as Arc<dyn Tool>);
        collection.add_tool(Arc::new(AskHumanTool::new()) as Arc<dyn Tool>);
        collection.add_tool(Arc::new(TerminateTool::new()) as Arc<dyn Tool>);
        b.iter(|| collection.to_definitions())
    });

    c.bench_function("tool_collection_get_tool", |b| {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(BashTool::new()) as Arc<dyn Tool>);
        b.iter(|| collection.get_tool(black_box("bash")))
    });
}

fn bench_llm_types(c: &mut Criterion) {
    use openmanus::llm::{LlmRequest, LlmResponse};
    use openmanus::schema::Message;

    c.bench_function("llm_request_new", |b| {
        b.iter(|| {
            LlmRequest::new(
                black_box("gpt-4"),
                vec![Message::user("Hello"), Message::assistant("Hi there!")],
            )
        })
    });

    c.bench_function("llm_request_serialization", |b| {
        let request = LlmRequest::new(
            "gpt-4",
            vec![Message::user("Hello"), Message::assistant("Hi there!")],
        );
        b.iter(|| serde_json::to_string(black_box(&request)))
    });

    c.bench_function("llm_response_deserialization", |b| {
        let json = r#"{
            "id": "test-id",
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }]
        }"#;
        b.iter(|| serde_json::from_str::<LlmResponse>(black_box(json)))
    });
}

fn bench_context_operations(c: &mut Criterion) {
    use openmanus::context::Context;
    use openmanus::schema::Message;

    c.bench_function("context_new", |b| b.iter(Context::new));

    c.bench_function("context_set_env", |b| {
        let mut ctx = Context::new();
        b.iter(|| {
            ctx.set_env(black_box("KEY"), black_box("value"));
        })
    });

    c.bench_function("context_recent_messages", |b| {
        let mut ctx = Context::new();
        let mut messages = Vec::new();
        for i in 0..50 {
            messages.push(Message::user(format!("Message {}", i)));
        }
        ctx.set_messages(messages);
        b.iter(|| ctx.recent_messages(black_box(10)))
    });
}

fn bench_schema_types(c: &mut Criterion) {
    use openmanus::schema::{Message, Role, ToolCall};

    c.bench_function("role_to_string", |b| {
        b.iter(|| black_box(Role::User).to_string())
    });

    c.bench_function("tool_call_new", |b| {
        b.iter(|| {
            ToolCall::new(
                black_box("call_123"),
                black_box("bash"),
                black_box(r#"{"command":"ls"}"#),
            )
        })
    });

    c.bench_function("message_serialization", |b| {
        let msg = Message::user("Hello, world!");
        b.iter(|| serde_json::to_string(black_box(&msg)))
    });

    c.bench_function("message_deserialization", |b| {
        let json = r#"{"role":"user","content":"Hello"}"#;
        b.iter(|| serde_json::from_str::<Message>(black_box(json)))
    });
}

criterion_group!(
    benches,
    bench_message_creation,
    bench_memory_operations,
    bench_tool_collection,
    bench_llm_types,
    bench_context_operations,
    bench_schema_types,
);
criterion_main!(benches);
