//! Protocol implementations for OpenManus
//!
//! This module provides protocol implementations for inter-agent communication
//! and external integrations.

pub mod a2a;

pub use a2a::{
    A2AAgent, A2ABrowser, A2ACard, A2ACapabilities, A2AManus, A2AMessage, A2APart, A2AServer,
    A2ASkill, A2ATask, A2ATaskStatus, A2AArtifact,
};
