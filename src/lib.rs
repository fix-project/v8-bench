#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(box_as_ptr)]
#![feature(naked_functions)]

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use serde::Serialize;

pub mod arca;
pub mod clone;
pub mod v8;
pub mod wasm2c;
pub mod wasmtime;

#[derive(Debug, Copy, Clone, Serialize)]
pub struct Datum {
    pub parallel: usize,
    pub iterations: usize,
    pub duration_ns: u128,
    pub debug: bool,
}

pub trait Benchmark {
    fn bench(&self, parallel: usize, warmup: Duration, duration: Duration) -> Vec<usize>;

    fn experiment(
        &self,
        parallel: usize,
        warmup: Duration,
        duration: Duration,
        run_as_process: bool,
    ) -> Vec<Datum> {
        let results = if run_as_process {
            self.bench(1, warmup, duration)
        } else {
            self.bench(parallel, warmup, duration)
        };
        let duration_ns = duration.as_nanos();

        let sum: usize = results.iter().sum::<usize>();
        let rate = sum as f64 / duration.as_secs_f64();
        if sum != 0 {
            let frequency = duration / sum as u32;
            println!("{parallel:4} threads: {rate:9.2} iters/second ({frequency:?} per iteration)",);
        } else {
            println!("{parallel:4} threads: {rate:9.2} iters/second (>1s per iteration)",);
        }

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
        run_as_process: bool,
    ) -> Vec<Datum> {
        let mut data = vec![];
        if run_as_process {
            data.extend(self.experiment(max_parallel, warmup, duration, run_as_process))
        } else {
            let lg_max_parallel = max_parallel.ilog2();
            for lg_parallel in 0..lg_max_parallel + 1 {
                let parallel = 1 << lg_parallel;
                data.extend(self.experiment(parallel, warmup, duration, run_as_process));
            }
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
        notup: &AtomicUsize,
        notready: &AtomicUsize,
        notdone: &AtomicUsize,
    ) -> usize;
}

impl<T: SimpleRuntime> SingleThreadedRuntime for T {
    fn run(
        &self,
        warmup: Duration,
        duration: Duration,
        notup: &AtomicUsize,
        notready: &AtomicUsize,
        notdone: &AtomicUsize,
    ) -> usize {
        let mut state = self.setup();
        notup.fetch_sub(1, Ordering::Release);
        while notup.load(Ordering::Acquire) != 0 {}
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
        let notup = Box::new(AtomicUsize::new(parallel));
        let notready = Box::new(AtomicUsize::new(parallel));
        let notdone = Box::new(AtomicUsize::new(parallel));
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| self.run(warmup, duration, &notup, &notready, &notdone));
                handles.push(handle);
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        })
    }
}
