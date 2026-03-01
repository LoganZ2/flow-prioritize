use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::state::SchedulerState;

pub async fn metrics_loop(state: Arc<Mutex<SchedulerState>>, interval: Duration, c: usize) {
    let mut interval_ticker = tokio::time::interval(interval);
    let alpha = 0.5; // EWMA weight
    let interval_secs = interval.as_secs_f64();
    
    loop {
        interval_ticker.tick().await;
        
        let mut s = state.lock().unwrap();
        
        let current_lambda_m = s.interval_mouse_arrivals as f64 / interval_secs;
        let current_lambda_e = s.interval_elephant_arrivals as f64 / interval_secs;
        let current_mu_total = s.interval_tasks_completed as f64 / interval_secs;
        
        let current_mu_per_worker = if current_mu_total > 0.0 {
            current_mu_total / c as f64
        } else {
            s.mu_per_worker
        };
        
        // EWMA update for smooth transitions
        s.lambda_mouse = alpha * current_lambda_m + (1.0 - alpha) * s.lambda_mouse;
        s.lambda_elephant = alpha * current_lambda_e + (1.0 - alpha) * s.lambda_elephant;
        if s.interval_tasks_completed > 0 {
            s.mu_per_worker = alpha * current_mu_per_worker + (1.0 - alpha) * s.mu_per_worker;
        }
        
        s.interval_mouse_arrivals = 0;
        s.interval_elephant_arrivals = 0;
        s.interval_tasks_completed = 0;
        
        let total_lambda = s.lambda_mouse + s.lambda_elephant;
        let capacity = c as f64 * s.mu_per_worker;
        
        // Compute Load rho
        if capacity > 0.0 {
            s.rho = total_lambda / capacity;
        } else if total_lambda > 0.0 {
            s.rho = 1.0; 
        } else {
            s.rho = 0.0;
        }
    }
}
