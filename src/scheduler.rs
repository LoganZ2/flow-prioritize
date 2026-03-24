use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

use crate::state::SchedulerState;
use crate::types::FlowType;
use crate::worker::worker_loop;
use crate::metrics::metrics_loop;

/// A priority-aware scheduler for mixed asynchronous workloads.
///
/// The scheduler separates tasks into mouse and elephant queues and
/// dynamically adjusts dispatch behavior based on estimated runtime load.
pub struct FlowPrioritizeScheduler {
    state: Arc<Mutex<SchedulerState>>,
    semaphore: Arc<Semaphore>,
    workers: Vec<JoinHandle<()>>,
    metrics_task: JoinHandle<()>,
    c: usize,
}

impl FlowPrioritizeScheduler {
    /// Creates a new scheduler instance.
    ///
    /// - `c`: number of worker tasks
    /// - `rho_low`: threshold between light and heavy load scheduling
    /// - `metrics_interval`: interval for telemetry updates
    pub fn new(c: usize, rho_low: f64, metrics_interval: Duration) -> Self {
        let state = Arc::new(Mutex::new(SchedulerState::new(200.0)));
        let semaphore = Arc::new(Semaphore::new(0));
        
        let mut workers = Vec::with_capacity(c);
        for _ in 0..c {
            let state_clone = Arc::clone(&state);
            let sem_clone = Arc::clone(&semaphore);
            let worker = tokio::spawn(async move {
                worker_loop(state_clone, sem_clone, rho_low).await;
            });
            workers.push(worker);
        }
        
        let state_clone = Arc::clone(&state);
        let metrics_task = tokio::spawn(async move {
            metrics_loop(state_clone, metrics_interval, c).await;
        });
        
        Self {
            state,
            semaphore,
            workers,
            metrics_task,
            c,
        }
    }

    /// Submits a task with flow classification.
    ///
    /// Returns an error when the scheduler is shut down or when overload
    /// admission control rejects an elephant task.
    pub fn submit<F>(&self, flow_type: FlowType, task: F) -> Result<(), &'static str>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut state = self.state.lock().unwrap();
        
        if state.is_shutdown {
            return Err("Scheduler is shut down");
        }
        
        // Admission control logic (Overload)
        if state.rho >= 1.0 && matches!(flow_type, FlowType::Elephant) {
            return Err("Overload: Elephant flow rejected");
        }
        
        let boxed_task = Box::pin(task);
        
        match flow_type {
            FlowType::Mouse => {
                state.interval_mouse_arrivals += 1;
                state.mouse_queue.push_back(boxed_task);
            }
            FlowType::Elephant => {
                state.interval_elephant_arrivals += 1;
                state.elephant_queue.push_back(boxed_task);
            }
        }
        
        self.semaphore.add_permits(1);
        Ok(())
    }
    
    /// Returns `(rho, lambda_mouse, lambda_elephant, mu_total)`.
    pub fn get_metrics(&self) -> (f64, f64, f64, f64) {
        let state = self.state.lock().unwrap();
        (state.rho, state.lambda_mouse, state.lambda_elephant, state.mu_per_worker * self.c as f64)
    }

    /// Returns `(mouse_queue_len, elephant_queue_len)`.
    pub fn get_queue_lengths(&self) -> (usize, usize) {
        let state = self.state.lock().unwrap();
        (state.mouse_queue.len(), state.elephant_queue.len())
    }

    /// Gracefully shuts down the scheduler and waits for worker completion.
    pub async fn shutdown(self) {
        {
            let mut state = self.state.lock().unwrap();
            state.is_shutdown = true;
        }
        
        self.semaphore.add_permits(self.c);
        
        for w in self.workers {
            let _ = w.await;
        }
        
        self.metrics_task.abort();
    }
}
