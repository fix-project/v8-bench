use std::time::Duration;

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
    fn run(&self, parallel: usize, iterations_per_thread: usize) -> Duration;

    fn collect_data(&self, max_parallel: usize, iterations_per_thread: usize) -> Vec<Datum> {
        let mut data = vec![];
        let lg_max_parallel = max_parallel.ilog2();
        for lg_parallel in 0..lg_max_parallel + 1 {
            let parallel = 1 << lg_parallel;
            let duration = self.run(parallel, iterations_per_thread);
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
