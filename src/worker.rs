use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use crate::state::SchedulerState;

pub async fn worker_loop(state: Arc<Mutex<SchedulerState>>, semaphore: Arc<Semaphore>, rho_low: f64) {
    loop {
        let permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => break,
        };
        permit.forget(); 
        
        let task_opt = {
            let mut s = state.lock().unwrap();
            if s.is_shutdown && s.mouse_queue.is_empty() && s.elephant_queue.is_empty() {
                break;
            }
            
            let rho = s.rho;
            let mut selected = None;
            
            // Scheduling Decision based on Real-time Load (rho)
            if rho < rho_low {
                // Light load - Improve throughput for elephant flows, non-preemptive like behavior
                if let Some(t) = s.elephant_queue.pop_front() {
                    selected = Some(t);
                } else if let Some(t) = s.mouse_queue.pop_front() {
                    selected = Some(t);
                }
            } else {
                // Heavy load - Strict priority, minimizing mouse latency
                if let Some(t) = s.mouse_queue.pop_front() {
                    selected = Some(t);
                } else if let Some(t) = s.elephant_queue.pop_front() {
                    selected = Some(t);
                }
            }
            
            selected
        };
        
        if let Some(task) = task_opt {
            task.await;
            
            let mut s = state.lock().unwrap();
            s.interval_tasks_completed += 1;
        }
    }
}
