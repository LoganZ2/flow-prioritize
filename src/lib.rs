//! # FlowPrioritize
//! 
//! An optimized, model-driven dynamic scheduling mechanism for asynchronous 
//! Rust applications (running on Tokio), based on an M/M/c priority queue system.
//!
//! This library is specifically designed to handle mixed workloads common in 
//! online games and real-time distributed systems, separating lightweight, 
//! latency-sensitive tasks ("Mouse flows") from heavy, throughput-oriented tasks ("Elephant flows").

pub mod types;
pub mod state;
pub mod worker;
pub mod metrics;
pub mod scheduler;

pub use types::FlowType;
pub use scheduler::FlowPrioritizeScheduler;
