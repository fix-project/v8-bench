#![feature(allocator_api)]
#![feature(slice_ptr_get)]

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use serde::Serialize;

pub mod arca;
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
    fn bench(&self, parallel: usize, warmup: Duration, duration: Duration) -> Vec<usize>;

    fn experiment(&self, parallel: usize, warmup: Duration, duration: Duration) -> Vec<Datum> {
        let results = self.bench(parallel, warmup, duration);
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

    fn collect_data(
        &self,
        max_parallel: usize,
        warmup: Duration,
        duration: Duration,
    ) -> Vec<Datum> {
        let mut data = vec![];
        let lg_max_parallel = max_parallel.ilog2();
        for lg_parallel in 0..lg_max_parallel + 1 {
            let parallel = 1 << lg_parallel;
            data.extend(self.experiment(parallel, warmup, duration));
        }
        data
    }
}
pub trait SimpleRuntime {
    type State;

    fn setup(&self) -> Self::State;
    fn iterate(&self, state: &mut Self::State);
}

pub trait SingleThreadedRuntime {
    fn run(
        &self,
        warmup: Duration,
        duration: Duration,
        notready: &AtomicUsize,
        notdone: &AtomicUsize,
    ) -> usize;
}

impl<T: SimpleRuntime> SingleThreadedRuntime for T {
    fn run(
        &self,
        warmup: Duration,
        duration: Duration,
        notready: &AtomicUsize,
        notdone: &AtomicUsize,
    ) -> usize {
        let mut state = self.setup();
        let warmup_start = Instant::now();
        while warmup_start.elapsed() < warmup {
            self.iterate(&mut state);
        }
        notready.fetch_sub(1, Ordering::Release);
        while notready.load(Ordering::Acquire) != 0 {
            self.iterate(&mut state);
        }
        let start = Instant::now();
        let mut iters = 0;
        loop {
            self.iterate(&mut state);
            if start.elapsed() < duration {
                iters += 1;
            } else {
                break;
            }
        }
        notdone.fetch_sub(1, Ordering::Release);
        while notready.load(Ordering::Acquire) != 0 {
            self.iterate(&mut state);
        }
        iters
    }
}

impl<T: SingleThreadedRuntime + Sync> Benchmark for T {
    fn bench(&self, parallel: usize, warmup: Duration, duration: Duration) -> Vec<usize> {
        let notready = AtomicUsize::new(parallel);
        let notdone = AtomicUsize::new(parallel);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| self.run(warmup, duration, &notready, &notdone));
                handles.push(handle);
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        })
    }
}
