use std::future::Future;
use std::pin::Pin;

/// Categorizes the type of workload being submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowType {
    /// High-frequency, latency-sensitive tasks (e.g., control updates).
    Mouse,
    /// Low-frequency, throughput-oriented heavy tasks (e.g., database persistence).
    Elephant,
}

pub type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send>>;
