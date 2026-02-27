use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

use crate::state::SchedulerState;
use crate::types::FlowType;
use crate::worker::worker_loop;
use crate::metrics::metrics_loop;

pub struct FlowPrioritizeScheduler {
    state: Arc<Mutex<SchedulerState>>,
    semaphore: Arc<Semaphore>,
    workers: Vec<JoinHandle<()>>,
    metrics_task: JoinHandle<()>,
    c: usize,
}

impl FlowPrioritizeScheduler {
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
    
    pub fn get_metrics(&self) -> (f64, f64, f64, f64) {
        let state = self.state.lock().unwrap();
        (state.rho, state.lambda_mouse, state.lambda_elephant, state.mu_per_worker * self.c as f64)
    }

    pub fn get_queue_lengths(&self) -> (usize, usize) {
        let state = self.state.lock().unwrap();
        (state.mouse_queue.len(), state.elephant_queue.len())
    }

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
