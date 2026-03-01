//! Flow orchestration for multi-agent workflows
//!
//! This module provides flow orchestration for coordinating multiple agents:
//!
//! - [`BaseFlow`] - Simple single agent execution flow
//! - [`PlanningFlow`] - Decomposes tasks into plans and assigns to agents
//! - [`FlowFactory`] - Factory for creating flow instances
//! - [`FlowBuilder`] - Builder for complex flow configurations
//!
//! # Example
//!
//! ```rust,ignore
//! use openmanus::flow::{FlowFactory, FlowType, FlowBuilder};
//! use openmanus::agent::Manus;
//!
//! // Using factory
//! let manus = Manus::new().await?;
//! let flow = FlowFactory::base(Box::new(manus));
//!
//! // Using builder
//! let manus = Manus::new().await?;
//! let flow = FlowBuilder::new(FlowType::Base)
//!     .with_agent("main", Box::new(manus))
//!     .build()?;
//!
//! // Execute
//! let result = flow.execute("Hello world").await?;
//! ```

mod base;
mod factory;
mod planning;

pub use base::BaseFlow;
pub use factory::{FlowBuilder, FlowFactory, FlowType};
pub use planning::{PlanStep, PlanningFlow};

/// Re-export common flow types
pub mod prelude {
    pub use super::base::BaseFlow;
    pub use super::factory::{FlowBuilder, FlowFactory, FlowType};
    pub use super::planning::{PlanStep, PlanningFlow};
}
