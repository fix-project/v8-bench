#![feature(allocator_api)]
#![feature(slice_ptr_get)]

use std::{
    sync::atomic::{AtomicBool, Ordering},
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
    fn bench(&self, parallel: usize, duration: Duration) -> Vec<usize>;

    fn experiment(&self, parallel: usize, duration: Duration) -> Vec<Datum> {
        let results = self.bench(parallel, duration);
        let duration_ns = duration.as_nanos();

        let min = results.iter().min().unwrap();
        let max = results.iter().max().unwrap();
        let mean: usize = results.iter().sum::<usize>() / results.len();
        let range = core::cmp::max(max - mean, mean - min);
        let rate = mean as f64 / duration.as_secs_f64();
        println!(
            "{parallel:4} threads: {rate:9.2} iters/thread/second ({mean:9}±{range:<7} iters/thread in {duration:?})",
        );

        results
            .into_iter()
            .map(|iterations| Datum {
                debug: cfg!(debug_assertions),
                parallel,
                iterations,
                duration_ns,
            })
            .collect()
    }

    fn collect_data(&self, max_parallel: usize, duration: Duration) -> Vec<Datum> {
        let mut data = vec![];
        let lg_max_parallel = max_parallel.ilog2();
        for lg_parallel in 0..lg_max_parallel + 1 {
            let parallel = 1 << lg_parallel;
            data.extend(self.experiment(parallel, duration));
        }
        data
    }
}

pub trait SingleThreadedRuntime {
    fn run(&self, duration: Duration) -> usize;
}

impl<T: SingleThreadedRuntime + Sync> Benchmark for T {
    fn bench(&self, parallel: usize, duration: Duration) -> Vec<usize> {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.run(duration)
                });
                handles.push(handle);
            }
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        })
    }
}
