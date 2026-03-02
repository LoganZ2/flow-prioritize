use flow_prioritize::{FlowType, FlowPrioritizeScheduler};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::sync::mpsc;
use std::future::Future;
use std::pin::Pin;

#[derive(Serialize, Clone)]
struct TaskRecord {
    scenario: String,
    task_id: usize,
    flow_type: String,
    arrival_time_ms: u128,
    wait_time_ms: u128,
    execution_time_ms: u128,
    dropped: bool,
}

#[derive(Serialize, Clone)]
struct MetricsRecord {
    scenario: String,
    timestamp_ms: u128,
    rho: f64,
    lambda_mouse: f64,
    lambda_elephant: f64,
    mu_total: f64,
    mouse_queue_len: usize,
    elephant_queue_len: usize,
}

type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send>>;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

struct SjfTask {
    expected_duration: u64,
    task_id: usize,
    task: BoxedTask,
}

impl PartialEq for SjfTask {
    fn eq(&self, other: &Self) -> bool {
        self.expected_duration == other.expected_duration && self.task_id == other.task_id
    }
}
impl Eq for SjfTask {}
impl PartialOrd for SjfTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SjfTask {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expected_duration.cmp(&self.expected_duration).then_with(|| other.task_id.cmp(&self.task_id))
    }
}

#[tokio::main]
async fn main() {
    println!("==================================================");
    println!("Queue Theory Scheduler - Expanded Benchmark Suite");
    println!("==================================================\n");

    let num_workers = 4;
    let total_tasks = 2000; // per scenario, per scheduler

    let mut all_tasks = Vec::new();
    let mut all_metrics = Vec::new();

    // Scenario 1: Standard Game Server (90% Mouse, 10% Elephant)
    run_scenario("Scenario_1_GameServer", num_workers, total_tasks, 10, &mut all_tasks, &mut all_metrics).await;
    
    // Scenario 2: Balanced Microservice (50% Mouse, 50% Elephant)
    run_scenario("Scenario_2_Microservices", num_workers, total_tasks, 2, &mut all_tasks, &mut all_metrics).await;
    
    println!("\nExporting comprehensive experimental data...");

    let mut wtr_tasks = csv::Writer::from_path("experiment_data/all_tasks_trace.csv").unwrap();
    for record in all_tasks.iter() {
        wtr_tasks.serialize(record).unwrap();
    }
    wtr_tasks.flush().unwrap();

    let mut wtr_metrics = csv::Writer::from_path("experiment_data/all_system_metrics.csv").unwrap();
    for record in all_metrics.iter() {
        wtr_metrics.serialize(record).unwrap();
    }
    wtr_metrics.flush().unwrap();

    println!("Data exported successfully!");
}

async fn run_scenario(
    scenario_name: &str, 
    num_workers: usize, 
    total_tasks: usize, 
    mouse_ratio_mod: usize, 
    all_tasks: &mut Vec<TaskRecord>,
    all_metrics: &mut Vec<MetricsRecord>
) {
    println!(">>> RUNNING SCENARIO: {}", scenario_name);
    
    // 1. Proposed Model
    println!("  -> Testing Proposed Model...");
    let (mut p_tasks, mut p_metrics) = run_proposed_scheduler(num_workers, total_tasks, mouse_ratio_mod, format!("{}_Proposed", scenario_name)).await;
    all_tasks.append(&mut p_tasks);
    all_metrics.append(&mut p_metrics);

    // 2. FIFO Baseline
    println!("  -> Testing FIFO Baseline...");
    let mut f_tasks = run_baseline_scheduler(num_workers, total_tasks, mouse_ratio_mod, format!("{}_FIFO", scenario_name)).await;
    all_tasks.append(&mut f_tasks);

    // 3. SJF Baseline
    println!("  -> Testing SJF Baseline...");
    let mut s_tasks = run_sjf_scheduler(num_workers, total_tasks, mouse_ratio_mod, format!("{}_SJF", scenario_name)).await;
    all_tasks.append(&mut s_tasks);
}

async fn run_proposed_scheduler(num_workers: usize, total_tasks: usize, mouse_ratio_mod: usize, tag: String) -> (Vec<TaskRecord>, Vec<MetricsRecord>) {
    let rho_low = 0.4;
    let metrics_interval = Duration::from_millis(100);

    let scheduler = Arc::new(FlowPrioritizeScheduler::new(num_workers, rho_low, metrics_interval));
    let task_records = Arc::new(Mutex::new(Vec::new()));
    let metrics_records = Arc::new(Mutex::new(Vec::new()));
    let start_time = Instant::now();

    let scheduler_metrics_clone = Arc::clone(&scheduler);
    let metrics_data_clone = Arc::clone(&metrics_records);
    let tag_clone = tag.clone();
    
    let metrics_monitor = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let (rho, lm, le, mu) = scheduler_metrics_clone.get_metrics();
            let (q_mouse, q_elephant) = scheduler_metrics_clone.get_queue_lengths();
            
            let mut records = metrics_data_clone.lock().unwrap();
            records.push(MetricsRecord {
                scenario: tag_clone.clone(),
                timestamp_ms: start_time.elapsed().as_millis(),
                rho, lambda_mouse: lm, lambda_elephant: le, mu_total: mu,
                mouse_queue_len: q_mouse, elephant_queue_len: q_elephant,
            });
        }
    });

    for i in 0..total_tasks {
        let is_mouse = i % mouse_ratio_mod != 0; 
        let flow_type = if is_mouse { FlowType::Mouse } else { FlowType::Elephant };

        let submit_time = Instant::now();
        let arrival_time_ms = start_time.elapsed().as_millis();
        let task_records_clone = Arc::clone(&task_records);
        let tag_t = tag.clone();
        
        let task = async move {
            let wait_time = submit_time.elapsed();
            let work_duration = if is_mouse { Duration::from_millis(5) } else { Duration::from_millis(50) };
            let start_work = Instant::now();
            sleep(work_duration).await; 
            let execution_time = start_work.elapsed();

            let mut res = task_records_clone.lock().unwrap();
            res.push(TaskRecord {
                scenario: tag_t, task_id: i,
                flow_type: if is_mouse { "Mouse".to_string() } else { "Elephant".to_string() },
                arrival_time_ms, wait_time_ms: wait_time.as_millis(), execution_time_ms: execution_time.as_millis(), dropped: false,
            });
        };

        if let Err(_e) = scheduler.submit(flow_type, task) {
            task_records.lock().unwrap().push(TaskRecord {
                scenario: tag.clone(), task_id: i,
                flow_type: if is_mouse { "Mouse".to_string() } else { "Elephant".to_string() },
                arrival_time_ms, wait_time_ms: 0, execution_time_ms: 0, dropped: true,
            });
        }

        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let sleep_duration = if elapsed_secs < 3.0 { Duration::from_millis(10) } 
        else if elapsed_secs < 6.0 { Duration::from_millis(3) } 
        else { Duration::from_millis(1) };
        sleep(sleep_duration).await;
    }

    sleep(Duration::from_secs(2)).await;
    metrics_monitor.abort();
    let _ = metrics_monitor.await;
    let sched = Arc::try_unwrap(scheduler).map_err(|_| "Failed").unwrap();
    sched.shutdown().await;

    let tr = task_records.lock().unwrap().clone();
    let mr = metrics_records.lock().unwrap().clone();
    (tr, mr)
}

async fn run_baseline_scheduler(num_workers: usize, total_tasks: usize, mouse_ratio_mod: usize, tag: String) -> Vec<TaskRecord> {
    let (tx, mut rx) = mpsc::unbounded_channel::<BoxedTask>();
    let task_records = Arc::new(Mutex::new(Vec::new()));
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    let mut handles = vec![];
    
    for _ in 0..num_workers {
        let rx_clone = Arc::clone(&rx);
        let h = tokio::spawn(async move {
            loop {
                let task_opt = { let mut queue = rx_clone.lock().await; queue.recv().await };
                if let Some(task) = task_opt { task.await; } else { break; }
            }
        });
        handles.push(h);
    }

    let start_time = Instant::now();
    for i in 0..total_tasks {
        let is_mouse = i % mouse_ratio_mod != 0; 
        let submit_time = Instant::now();
        let arrival_time_ms = start_time.elapsed().as_millis();
        let task_records_clone = Arc::clone(&task_records);
        let tag_t = tag.clone();
        
        let task = Box::pin(async move {
            let wait_time = submit_time.elapsed();
            let work_duration = if is_mouse { Duration::from_millis(5) } else { Duration::from_millis(50) };
            let start_work = Instant::now();
            sleep(work_duration).await; 
            let execution_time = start_work.elapsed();

            task_records_clone.lock().unwrap().push(TaskRecord {
                scenario: tag_t, task_id: i,
                flow_type: if is_mouse { "Mouse".to_string() } else { "Elephant".to_string() },
                arrival_time_ms, wait_time_ms: wait_time.as_millis(), execution_time_ms: execution_time.as_millis(), dropped: false,
            });
        });

        let _ = tx.send(task);

        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let sleep_duration = if elapsed_secs < 3.0 { Duration::from_millis(10) } 
        else if elapsed_secs < 6.0 { Duration::from_millis(3) } 
        else { Duration::from_millis(1) };
        sleep(sleep_duration).await;
    }

    drop(tx);
    for h in handles { let _ = h.await; }
    task_records.lock().unwrap().clone()
}

async fn run_sjf_scheduler(num_workers: usize, total_tasks: usize, mouse_ratio_mod: usize, tag: String) -> Vec<TaskRecord> {
    let task_records = Arc::new(Mutex::new(Vec::new()));
    let sjf_queue = Arc::new(tokio::sync::Mutex::new(BinaryHeap::<SjfTask>::new()));
    let notify = Arc::new(tokio::sync::Notify::new());
    let processed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = vec![];
    
    for _ in 0..num_workers {
        let queue_clone = Arc::clone(&sjf_queue);
        let notify_clone = Arc::clone(&notify);
        let processed_clone = Arc::clone(&processed_count);
        
        let h = tokio::spawn(async move {
            loop {
                let task_opt = { let mut queue = queue_clone.lock().await; queue.pop() };
                if let Some(sjf_task) = task_opt {
                    sjf_task.task.await;
                    let current = processed_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if current + 1 >= total_tasks {
                        notify_clone.notify_waiters();
                        break;
                    }
                } else {
                    notify_clone.notified().await;
                    if processed_clone.load(std::sync::atomic::Ordering::SeqCst) >= total_tasks { break; }
                }
            }
        });
        handles.push(h);
    }

    let start_time = Instant::now();
    for i in 0..total_tasks {
        let is_mouse = i % mouse_ratio_mod != 0; 
        let submit_time = Instant::now();
        let arrival_time_ms = start_time.elapsed().as_millis();
        let task_records_clone = Arc::clone(&task_records);
        let tag_t = tag.clone();
        let expected_duration = if is_mouse { 5 } else { 50 };
        
        let task = Box::pin(async move {
            let wait_time = submit_time.elapsed();
            let work_duration = Duration::from_millis(expected_duration);
            let start_work = Instant::now();
            sleep(work_duration).await; 
            let execution_time = start_work.elapsed();

            task_records_clone.lock().unwrap().push(TaskRecord {
                scenario: tag_t, task_id: i,
                flow_type: if is_mouse { "Mouse".to_string() } else { "Elephant".to_string() },
                arrival_time_ms, wait_time_ms: wait_time.as_millis(), execution_time_ms: execution_time.as_millis(), dropped: false,
            });
        });

        {
            let mut queue = sjf_queue.lock().await;
            queue.push(SjfTask { expected_duration, task_id: i, task });
        }
        notify.notify_one();

        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let sleep_duration = if elapsed_secs < 3.0 { Duration::from_millis(10) } 
        else if elapsed_secs < 6.0 { Duration::from_millis(3) } 
        else { Duration::from_millis(1) };
        sleep(sleep_duration).await;
    }

    for h in handles { let _ = h.await; }
    task_records.lock().unwrap().clone()
}
