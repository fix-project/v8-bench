use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use crate::Benchmark;

use vmm::runtime::{Mmap, Runtime};

const KERNEL_ELF: &[u8] = include_bytes!(env!("CARGO_BIN_FILE_KBENCH_kbench"));

pub struct ArcaBenchmark {
    elf: &'static [u8],
}

impl ArcaBenchmark {
    pub fn new(elf: &'static [u8]) -> Self {
        ArcaBenchmark { elf }
    }
}

impl Benchmark for ArcaBenchmark {
    fn bench(&self, parallel: usize, warmup: Duration, duration: Duration) -> Vec<usize> {
        let mut mmap = Mmap::new(1 << 32);
        let cpus: usize = std::thread::available_parallelism().unwrap().into();
        let runtime = Runtime::new(cpus, &mut mmap, KERNEL_ELF.into());
        let value = {
            let allocator = runtime.allocator();
            let mut output = Vec::with_capacity_in(parallel, allocator);
            output.resize_with(parallel, || AtomicUsize::new(0));
            let output: Arc<[AtomicUsize], _> = output.into_boxed_slice().into();
            let inner_output = output.clone();
            let inner = Arc::into_raw(inner_output);
            let out_offset = allocator.to_offset(inner.as_ptr());
            let out_length = inner.len();
            assert_eq!(out_length, parallel);
            let mut new_elf = Vec::with_capacity_in(self.elf.len(), allocator);
            new_elf.extend_from_slice(self.elf);
            let new_elf = new_elf.into_boxed_slice();
            let ptr = new_elf.as_ptr();
            let len = new_elf.len();
            let offset = allocator.to_offset(ptr);
            let duration = duration.as_nanos().try_into().unwrap();
            let warmup = warmup.as_nanos().try_into().unwrap();
            runtime.run(&[offset, len, warmup, duration, out_offset, out_length]);
            output.iter().map(|x| x.load(Ordering::SeqCst)).collect()
        };
        std::mem::drop(runtime);
        value
    }
}
