use flow_prioritize::{FlowType, FlowPrioritizeScheduler};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("Starting Simple Game Server Scheduler Example...");

    // 1. Initialize the Scheduler
    // c = 2 workers
    // rho_low = 0.5 (Fair scheduling threshold)
    // Metrics update interval = 500ms
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(
        2, 
        0.5, 
        Duration::from_millis(500)
    ));

    // 2. Submit a lightweight task (e.g., Player position update)
    let _ = scheduler.submit(FlowType::Mouse, async {
        println!("[Mouse Flow] Updating player coordinates...");
        sleep(Duration::from_millis(2)).await; // Very fast
        println!("[Mouse Flow] Coordinates synced.");
    });

    // 3. Submit a heavyweight task (e.g., Database save)
    let _ = scheduler.submit(FlowType::Elephant, async {
        println!("[Elephant Flow] Starting background DB flush...");
        sleep(Duration::from_millis(50)).await; // Slow I/O
        println!("[Elephant Flow] DB flush complete.");
    });

    // Wait for tasks to finish
    sleep(Duration::from_millis(100)).await;

    // View current system load
    let (rho, _lm, _le, _mu) = scheduler.get_metrics();
    println!("Current System Load (rho): {:.2}", rho);

    // Shutdown
    let sched = Arc::try_unwrap(scheduler).map_err(|_| "Failed").unwrap();
    sched.shutdown().await;
    println!("Example finished.");
}