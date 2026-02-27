use std::collections::VecDeque;
use crate::types::BoxedTask;

pub struct SchedulerState {
    pub mouse_queue: VecDeque<BoxedTask>,
    pub elephant_queue: VecDeque<BoxedTask>,
    
    // Counters for the current interval
    pub interval_mouse_arrivals: usize,
    pub interval_elephant_arrivals: usize,
    pub interval_tasks_completed: usize,
    
    // EWMA Metrics
    pub lambda_mouse: f64,
    pub lambda_elephant: f64,
    pub mu_per_worker: f64,
    pub rho: f64,
    
    pub is_shutdown: bool,
}

impl SchedulerState {
    pub fn new(initial_mu: f64) -> Self {
        Self {
            mouse_queue: VecDeque::new(),
            elephant_queue: VecDeque::new(),
            interval_mouse_arrivals: 0,
            interval_elephant_arrivals: 0,
            interval_tasks_completed: 0,
            lambda_mouse: 0.0,
            lambda_elephant: 0.0,
            mu_per_worker: initial_mu,
            rho: 0.0,
            is_shutdown: false,
        }
    }
}
