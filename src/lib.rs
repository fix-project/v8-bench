#![feature(allocator_api)]

use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};

use serde::Serialize;

pub mod arca;
pub mod function;
pub mod v8;
pub mod wasm2c;

#[derive(Debug, Copy, Clone, Serialize)]
pub struct Datum {
    pub parallel: usize,
    pub iterations: usize,
    pub duration_ns: u128,
    pub debug: bool,
}

pub trait Benchmark {
    fn bench(&self, parallel: usize, duration: Duration) -> usize;

    fn experiment(&self, parallel: usize, duration: Duration) -> Datum {
        let iterations = self.bench(parallel, duration);
        let duration_ns = duration.as_nanos();

        let datum = Datum {
            debug: cfg!(debug_assertions),
            parallel,
            iterations,
            duration_ns,
        };

        let rate = iterations as f64 / duration.as_secs_f64();
        println!(
            "{parallel:4} threads: {rate:9.2} iters/second ({iterations:8} iters in {duration:?})",
        );
        datum
    }

    fn collect_data(&self, max_parallel: usize, duration: Duration) -> Vec<Datum> {
        let mut data = vec![];
        let lg_max_parallel = max_parallel.ilog2();
        for lg_parallel in 0..lg_max_parallel + 1 {
            let parallel = 1 << lg_parallel;
            data.push(self.experiment(parallel, duration));
        }
        data
    }
}

pub trait SingleThreadedRuntime {
    fn run(&self, duration: Duration) -> usize;
}

impl<T: SingleThreadedRuntime + Sync> Benchmark for T {
    fn bench(&self, parallel: usize, duration: Duration) -> usize {
        let begin = AtomicBool::new(false);
        let iterations = AtomicUsize::new(0);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    iterations.fetch_add(self.run(duration), Ordering::SeqCst);
                });
                handles.push(handle);
            }
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            for h in handles {
                h.join().unwrap();
            }
        });
        iterations.load(Ordering::SeqCst)
    }
}
