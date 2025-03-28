use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::Benchmark;

pub struct FunctionBenchmark;

fn add(x: u32, y: u32) -> u32 {
    x + y
}

impl FunctionBenchmark {
    pub fn new() -> Self {
        FunctionBenchmark
    }

    pub fn iterate(&self, todo: usize) {
        for _ in 0..todo {
            let x = core::hint::black_box(1);
            let y = core::hint::black_box(2);
            core::hint::black_box(add(x, y));
        }
    }
}

impl Default for FunctionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl Benchmark for FunctionBenchmark {
    fn run(&self, parallel: usize, iterations_per_thread: usize) -> Duration {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.iterate(iterations_per_thread);
                });
                handles.push(handle);
            }
            let start = Instant::now();
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            for h in handles {
                h.join().unwrap();
            }
            start.elapsed()
        })
    }
}
