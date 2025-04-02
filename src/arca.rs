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
    fn bench(&self, parallel: usize, duration: Duration) -> usize {
        let mut mmap = Mmap::new(1 << 32);
        let cpus: usize = std::thread::available_parallelism().unwrap().into();
        let runtime = Runtime::new(cpus, &mut mmap, KERNEL_ELF.into());
        let value = {
            let allocator = runtime.allocator();
            let output = Arc::new_in(AtomicUsize::new(0), allocator);
            let inner_output = output.clone();
            let out_offset = allocator.to_offset(Arc::into_raw(inner_output));
            let mut new_elf = Vec::with_capacity_in(self.elf.len(), allocator);
            new_elf.extend_from_slice(self.elf);
            let new_elf = new_elf.into_boxed_slice();
            let ptr = new_elf.as_ptr();
            let len = new_elf.len();
            let offset = allocator.to_offset(ptr);
            let duration = duration.as_nanos().try_into().unwrap();
            runtime.run(&[offset, len, parallel, duration, out_offset]);
            output.load(Ordering::SeqCst)
        };
        std::mem::drop(runtime);
        value
    }
}
