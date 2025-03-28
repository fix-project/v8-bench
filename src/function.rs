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
    fn run(&self, todo: usize) {
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
