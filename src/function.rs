use std::time::{Duration, Instant};

use crate::SingleThreadedRuntime;

pub struct FunctionBenchmark;

fn add(x: u32, y: u32) -> u32 {
    x + y
}

impl FunctionBenchmark {
    pub fn new() -> Self {
        FunctionBenchmark
    }
}

impl SingleThreadedRuntime for FunctionBenchmark {
    fn run(&self, duration: Duration) -> usize {
        let start = Instant::now();
        let mut i = 0;
        while start.elapsed() < duration {
            let x = core::hint::black_box(1);
            let y = core::hint::black_box(2);
            core::hint::black_box(add(x, y));
            i += 1;
        }
        i
    }
}

impl Default for FunctionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}
