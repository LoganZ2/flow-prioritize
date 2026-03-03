use flow_prioritize::{FlowType, FlowPrioritizeScheduler};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_basic_submission_and_execution() {
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(2, 0.4, Duration::from_millis(50)));
    let counter = Arc::new(AtomicUsize::new(0));

    // Submit a mouse flow
    let counter_clone = Arc::clone(&counter);
    let res1 = scheduler.submit(FlowType::Mouse, async move {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });
    assert!(res1.is_ok());

    // Submit an elephant flow
    let counter_clone2 = Arc::clone(&counter);
    let res2 = scheduler.submit(FlowType::Elephant, async move {
        counter_clone2.fetch_add(10, Ordering::SeqCst);
    });
    assert!(res2.is_ok());

    // Wait for execution
    sleep(Duration::from_millis(200)).await;

    // Both should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 11);

    Arc::try_unwrap(scheduler).map_err(|_| "unwrap failed").unwrap().shutdown().await;
}

#[tokio::test]
async fn test_admission_control_on_overload() {
    // Very low capacity scheduler to quickly trigger overload (rho >= 1.0)
    // We set metrics interval low to update rho fast
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(1, 0.2, Duration::from_millis(10)));
    
    // Flood the system with tasks to artificially pump lambda and rho
    for _ in 0..500 {
        let _ = scheduler.submit(FlowType::Mouse, async {
            sleep(Duration::from_millis(5)).await;
        });
    }
    
    // Wait slightly to let the metrics thread calculate the overload
    sleep(Duration::from_millis(50)).await;
    
    let (rho, _, _, _) = scheduler.get_metrics();
    // System should definitely be overloaded now
    assert!(rho >= 1.0, "System did not trigger overload state. Rho: {}", rho);

    // Now submit an Elephant flow, it should be REJECTED
    let res_elephant = scheduler.submit(FlowType::Elephant, async {});
    assert!(res_elephant.is_err(), "Elephant flow was not rejected during overload");
    assert_eq!(res_elephant.err().unwrap(), "Overload: Elephant flow rejected");

    // But Mouse flows should still be accepted (protected)
    let res_mouse = scheduler.submit(FlowType::Mouse, async {});
    assert!(res_mouse.is_ok(), "Mouse flow was incorrectly rejected during overload");

    // We don't cleanly shutdown here to save test time since workers are clogged,
    // Tokio test runtime will drop it.
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let scheduler = Arc::new(FlowPrioritizeScheduler::new(4, 0.4, Duration::from_millis(100)));
    
    let sched = Arc::try_unwrap(scheduler).map_err(|_| "unwrap failed").unwrap();
    // Assuming shutdown resolves and gracefully cleans up resources
    sched.shutdown().await;
    // If it reaches here without hanging, the test passes
}
