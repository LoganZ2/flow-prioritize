# FlowPrioritize

A highly optimized, model-driven dynamic scheduling mechanism for asynchronous Rust applications. Built entirely on top of the [Tokio](https://tokio.rs/) runtime, it abstracts mixed-workload processing as an **M/M/c Priority Queue**.

This software was explicitly designed to solve the *head-of-line blocking* problem in online multiplayer games and high-throughput real-time APIs, where lightweight, latency-sensitive packets ("Mouse flows") compete for computational resources with large, infrequent, throughput-heavy tasks ("Elephant flows").

## Features

- **Semantic Flow Classification**: Exposes a clean `submit` API allowing developers to explicitly label tasks as `FlowType::Mouse` or `FlowType::Elephant`.
- **Zero-Cost Telemetry (System Sensing)**: Uses a background detached thread to calculate the Exponentially Weighted Moving Average (EWMA) of arrival rates (lambda) and service rates (mu), calculating the real-time system load (rho) without locking the fast path.
- **Dynamic Scheduling Policies**:
  - *Light Load (rho < rho_low)*: Fairness-based processing.
  - *Heavy Load (rho >= rho_low)*: Strict non-preemptive priority to guarantee sub-millisecond dispatch for Mouse flows.
  - *Overload (rho >= 1.0)*: Active **Admission Control**, rejecting non-critical Elephant flows to prevent system starvation and out-of-memory (OOM) crashes.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
# Referencing the library directly from GitHub for the latest version
flow-prioritize = { git = "https://github.com/LoganZ2/flow-prioritize" }
tokio = { version = "1.49.0", features = ["full"] }
```

### Basic Example

```rust
use flow-prioritize::{FlowType, FlowPrioritizeScheduler};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // 4 workers, transitions at 40% load, 100ms sensing interval
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(4, 0.4, Duration::from_millis(100)));

    // Submit a lightweight synchronization task
    scheduler.submit(FlowType::Mouse, async {
        // ... fast game logic ...
    }).unwrap();

    // Submit a heavy DB task
    scheduler.submit(FlowType::Elephant, async {
        // ... heavy database I/O ...
    }).unwrap();

    // Clean shutdown
    Arc::try_unwrap(scheduler).map_err(|_| "Failed").unwrap().shutdown().await;
}
```

## Running the Academic Benchmark

This repository includes a rigorous benchmarking suite used to validate the model against theoretical M/M/c limits and against SJF/FIFO baselines.

```bash
cargo run --release --bin benchmark
```

This will generate 3 CSV files capturing the execution traces of over 3000 async tasks across varying burst phases. You can then plot the resulting latency CDF using the included Python script:

```bash
python3 -m venv venv
source venv/bin/activate
pip install pandas matplotlib seaborn numpy
python plot_results.py
```

