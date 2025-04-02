#![no_main]
#![no_std]
#![feature(ptr_metadata)]

use kernel::kvmclock;
use kernel::macros::kmain;
use kernel::prelude::*;
use kernel::rt;

extern crate alloc;

use alloc::{sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;

#[kmain]
async fn kmain(argv: &[usize]) {
    let &[offset, len, duration, output_offset, output_length] = argv else {
        todo!();
    };
    let parallel = output_length;
    let ptr: *mut u8 = PHYSICAL_ALLOCATOR.from_offset(offset);
    let output: Arc<[AtomicUsize]> = unsafe {
        Arc::from_raw(core::ptr::from_raw_parts(
            PHYSICAL_ALLOCATOR.from_offset::<AtomicUsize>(output_offset),
            output_length,
        ))
    };
    let elf = unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        let mut v = Vec::with_capacity(slice.len());
        v.extend_from_slice(slice);
        v
    };
    let thunk = Thunk::from_elf(&elf);
    let result = thunk.run();
    let Value::Lambda(lambda) = result else {
        panic!("expected lambda, got {result:x?}");
    };
    let duration = Duration::from_nanos(duration as u64);

    let mut set = Vec::with_capacity(parallel);
    for _ in 0..parallel {
        set.push(rt::spawn(run(duration, lambda.clone())));
    }
    for (x, y) in set.into_iter().zip(output.iter()) {
        y.store(x.await, Ordering::SeqCst);
    }
}

async fn run(duration: Duration, lambda: Lambda) -> usize {
    let start = kvmclock::time_since_boot();
    let mut i = 0;
    let timeslice = Duration::from_millis(50);
    let mut last_yield = start;
    loop {
        let now = kvmclock::time_since_boot();
        if now - start >= duration {
            break;
        }
        let lambda = core::hint::black_box(lambda.clone());
        let thunk = lambda.apply(Value::Tree(vec![Value::Word(1), Value::Word(2)].into()));
        core::hint::black_box(thunk.run());
        i += 1;
        if now - last_yield >= timeslice {
            rt::yield_now().await;
            last_yield = kvmclock::time_since_boot();
        }
    }
    i
}
