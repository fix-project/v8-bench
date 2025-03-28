use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use serde::Serialize;

pub mod function;
pub mod v8;

#[derive(Debug, Copy, Clone, Serialize)]
pub struct Datum {
    pub parallel: usize,
    pub iterations: usize,
    pub duration_ns: u128,
    pub debug: bool,
}

pub trait Benchmark {
    fn bench(&self, parallel: usize, iterations_per_thread: usize) -> Duration;

    fn collect_data(&self, max_parallel: usize, iterations_per_thread: usize) -> Vec<Datum> {
        let mut data = vec![];
        let lg_max_parallel = max_parallel.ilog2();
        for lg_parallel in 0..lg_max_parallel + 1 {
            let parallel = 1 << lg_parallel;
            let duration = self.bench(parallel, iterations_per_thread);
            let duration_ns = duration.as_nanos();
            let iterations = iterations_per_thread * parallel;

            data.push(Datum {
                debug: cfg!(debug_assertions),
                parallel,
                iterations,
                duration_ns,
            });

            let rate = iterations as f64 / duration.as_secs_f64();
            println!(
                "{parallel:4} threads: {rate:9.2} iters/second ({iterations:8} iters in {duration:?})",
            );
        }
        data
    }
}

pub trait SingleThreadedRuntime {
    fn run(&self, iterations: usize);
}

impl<T: SingleThreadedRuntime + Sync> Benchmark for T {
    fn bench(&self, parallel: usize, iterations_per_thread: usize) -> Duration {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.run(iterations_per_thread);
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
