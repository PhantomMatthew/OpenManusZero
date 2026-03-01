//! Memory management for agents

use crate::schema::Message;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;

/// Maximum number of messages in memory by default
const DEFAULT_MAX_MESSAGES: usize = 100;

/// Short-term memory using a ring buffer
#[derive(Debug)]
pub struct ShortTermMemory {
    messages: VecDeque<Message>,
    max_messages: usize,
}

impl ShortTermMemory {
    /// Create a new short-term memory with default capacity
    pub fn new() -> Self {
        Self {
            messages: VecDeque::with_capacity(DEFAULT_MAX_MESSAGES),
            max_messages: DEFAULT_MAX_MESSAGES,
        }
    }

    /// Create a new short-term memory with custom capacity
    pub fn with_capacity(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_messages),
            max_messages,
        }
    }

    /// Add a message to memory
    pub fn add(&mut self, message: Message) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }

    /// Add multiple messages to memory
    pub fn add_all(&mut self, messages: Vec<Message>) {
        for msg in messages {
            self.add(msg);
        }
    }

    /// Get all messages
    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }

    /// Get the last n messages
    pub fn recent(&self, n: usize) -> Vec<&Message> {
        let start = self.messages.len().saturating_sub(n);
        self.messages.iter().skip(start).collect()
    }

    /// Get the number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get the last message
    pub fn last(&self) -> Option<&Message> {
        self.messages.back()
    }

    /// Convert to a vector of messages
    pub fn to_vec(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }

    /// Convert to a vector of dictionaries
    pub fn to_dict_list(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect()
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe memory wrapper
#[derive(Debug, Clone)]
pub struct Memory {
    inner: Arc<RwLock<ShortTermMemory>>,
}

impl Memory {
    /// Create a new thread-safe memory
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ShortTermMemory::new())),
        }
    }

    /// Create a new thread-safe memory with custom capacity
    pub fn with_capacity(max_messages: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ShortTermMemory::with_capacity(max_messages))),
        }
    }

    /// Add a message to memory
    pub fn add(&self, message: Message) {
        self.inner.write().add(message);
    }

    /// Add multiple messages to memory
    pub fn add_all(&self, messages: Vec<Message>) {
        self.inner.write().add_all(messages);
    }

    /// Get all messages (cloned)
    pub fn messages(&self) -> Vec<Message> {
        self.inner.read().to_vec()
    }

    /// Get the last n messages
    pub fn recent(&self, n: usize) -> Vec<Message> {
        self.inner.read().recent(n).into_iter().cloned().collect()
    }

    /// Get the number of messages
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Clear all messages
    pub fn clear(&self) {
        self.inner.write().clear();
    }

    /// Get the last message
    pub fn last(&self) -> Option<Message> {
        self.inner.read().last().cloned()
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

/// Long-term memory trait (for persistent storage)
#[async_trait::async_trait]
pub trait LongTermMemory: Send + Sync {
    /// Store a value with a key
    async fn store(&self, key: &str, value: &serde_json::Value) -> crate::Result<()>;

    /// Retrieve a value by key
    async fn retrieve(&self, key: &str) -> crate::Result<Option<serde_json::Value>>;

    /// Delete a value by key
    async fn delete(&self, key: &str) -> crate::Result<()>;

    /// List all keys
    async fn list_keys(&self) -> crate::Result<Vec<String>>;

    /// Clear all stored values
    async fn clear(&self) -> crate::Result<()>;
}

/// In-memory long-term storage (for testing)
#[allow(dead_code)]
pub struct InMemoryLongTermMemory {
    storage: Arc<RwLock<std::collections::HashMap<String, serde_json::Value>>>,
}

#[allow(dead_code)]
impl InMemoryLongTermMemory {
    /// Create a new in-memory long-term memory
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for InMemoryLongTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LongTermMemory for InMemoryLongTermMemory {
    async fn store(&self, key: &str, value: &serde_json::Value) -> crate::Result<()> {
        self.storage.write().insert(key.to_string(), value.clone());
        Ok(())
    }

    async fn retrieve(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
        Ok(self.storage.read().get(key).cloned())
    }

    async fn delete(&self, key: &str) -> crate::Result<()> {
        self.storage.write().remove(key);
        Ok(())
    }

    async fn list_keys(&self) -> crate::Result<Vec<String>> {
        Ok(self.storage.read().keys().cloned().collect())
    }

    async fn clear(&self) -> crate::Result<()> {
        self.storage.write().clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_term_memory_new() {
        let mem = ShortTermMemory::new();
        assert!(mem.is_empty());
        assert_eq!(mem.len(), 0);
    }

    #[test]
    fn test_short_term_memory_add() {
        let mut mem = ShortTermMemory::new();
        mem.add(Message::user("Hello"));
        assert!(!mem.is_empty());
        assert_eq!(mem.len(), 1);
    }

    #[test]
    fn test_short_term_memory_max_capacity() {
        let mut mem = ShortTermMemory::with_capacity(3);
        mem.add(Message::user("msg1"));
        mem.add(Message::user("msg2"));
        mem.add(Message::user("msg3"));
        mem.add(Message::user("msg4"));

        assert_eq!(mem.len(), 3);
        let msgs = mem.to_vec();
        assert_eq!(msgs[0].content, Some("msg2".to_string()));
        assert_eq!(msgs[2].content, Some("msg4".to_string()));
    }

    #[test]
    fn test_short_term_memory_recent() {
        let mut mem = ShortTermMemory::new();
        mem.add(Message::user("msg1"));
        mem.add(Message::user("msg2"));
        mem.add(Message::user("msg3"));

        let recent = mem.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, Some("msg2".to_string()));
        assert_eq!(recent[1].content, Some("msg3".to_string()));
    }

    #[test]
    fn test_short_term_memory_clear() {
        let mut mem = ShortTermMemory::new();
        mem.add(Message::user("msg1"));
        mem.add(Message::user("msg2"));
        mem.clear();
        assert!(mem.is_empty());
    }

    #[test]
    fn test_short_term_memory_last() {
        let mut mem = ShortTermMemory::new();
        assert!(mem.last().is_none());
        mem.add(Message::user("msg1"));
        mem.add(Message::user("msg2"));
        let last = mem.last().unwrap();
        assert_eq!(last.content, Some("msg2".to_string()));
    }

    #[test]
    fn test_memory_thread_safe() {
        let mem = Memory::new();
        mem.add(Message::user("msg1"));

        let mem_clone = mem.clone();
        mem_clone.add(Message::user("msg2"));

        assert_eq!(mem.len(), 2);
    }

    #[test]
    fn test_memory_concurrent_access() {
        use std::thread;

        let mem = Arc::new(Memory::new());
        let mut handles = vec![];

        for i in 0..10 {
            let mem_clone = Arc::clone(&mem);
            handles.push(thread::spawn(move || {
                mem_clone.add(Message::user(format!("msg{}", i)));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(mem.len(), 10);
    }

    #[tokio::test]
    async fn test_in_memory_long_term_memory() {
        let mem = InMemoryLongTermMemory::new();

        // Store
        let value = serde_json::json!({"key": "value"});
        mem.store("test_key", &value).await.unwrap();

        // Retrieve
        let retrieved = mem.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), value);

        // List keys
        let keys = mem.list_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"test_key".to_string()));

        // Delete
        mem.delete("test_key").await.unwrap();
        let retrieved = mem.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_none());

        // Clear
        mem.store("key1", &serde_json::json!(1)).await.unwrap();
        mem.store("key2", &serde_json::json!(2)).await.unwrap();
        mem.clear().await.unwrap();
        let keys = mem.list_keys().await.unwrap();
        assert!(keys.is_empty());
    }
}
