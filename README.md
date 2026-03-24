# FlowPrioritize

FlowPrioritize is a Rust library for **priority-aware scheduling of mixed asynchronous workloads** on top of the [Tokio runtime](https://tokio.rs/).

It targets systems where low-latency tasks ("mouse flows") and heavy background tasks ("elephant flows") compete for the same executor resources. The scheduler uses real-time telemetry and queueing-inspired logic to reduce head-of-line blocking and protect tail latency.

## Key Features

- Explicit workload classification with `FlowType::Mouse` and `FlowType::Elephant`
- Dynamic scheduling based on real-time load (`rho`)
- EWMA telemetry for `lambda` and `mu` estimation
- Overload admission control (`rho >= 1.0`) for elephant flows
- Tokio-native API and lightweight integration

## Installation

### 1) Add dependency

```toml
[dependencies]
flow-prioritize = { git = "https://github.com/LoganZ2/flow-prioritize" }
tokio = { version = "1.49", features = ["full"] }
```

### 2) Minimal example

```rust
use flow_prioritize::{FlowPrioritizeScheduler, FlowType};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(
        4,
        0.4,
        Duration::from_millis(100),
    ));

    scheduler.submit(FlowType::Mouse, async {
        // latency-sensitive task
    }).unwrap();

    scheduler.submit(FlowType::Elephant, async {
        // heavy background task
    }).unwrap();

    let sched = Arc::try_unwrap(scheduler).unwrap();
    sched.shutdown().await;
}
```

See `examples/simple_game_server.rs` for a full runnable example.

## Reproducibility: Benchmark and Plots

### Run benchmark

```bash
cargo run --release --bin benchmark
```

This writes CSV outputs under `experiment_data/`.

### Generate plots

```bash
python3 -m venv venv
source venv/bin/activate
pip install pandas matplotlib seaborn numpy
python scripts/plot_results.py
```

Figures are written to `scripts/`.

## Development

```bash
cargo build
cargo test
```

CI config is available at `.github/workflows/ci.yml`.

## Repository Layout

- `src/`: library implementation
- `src/bin/benchmark.rs`: benchmark runner used in the paper
- `examples/`: integration example(s)
- `tests/`: integration tests
- `scripts/`: plotting and analysis scripts

## License

This project is licensed under the MIT License. See `LICENCE.txt`.

## Support

For questions about the software or reproducibility package, contact:

- `logan.zhuang@mail.utoronto.ca`

## SoftwareX Checklist Notes

- Open-source code on public GitHub: yes
- README with build/run/reproducibility instructions: yes
- License file in repository root (`LICENCE.txt`): yes
