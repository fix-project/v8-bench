#![no_main]
#![no_std]
#![feature(ptr_metadata)]

use alloc::collections::btree_map::BTreeMap;
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
    let &[
        offset,
        len,
        warmup,
        duration,
        output_offset,
        output_length,
        tlb_shootdowns,
    ] = argv
    else {
        todo!();
    };
    kernel::set_tlb_shootdowns_enabled(tlb_shootdowns == 1);
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
    let warmup = Duration::from_nanos(warmup as u64);
    let duration = Duration::from_nanos(duration as u64);

    let mut set = Vec::with_capacity(parallel);
    let notready = Arc::new(AtomicUsize::new(parallel));
    let notdone = Arc::new(AtomicUsize::new(parallel));
    kernel::profile::reset();
    kernel::profile::begin();
    for _ in 0..parallel {
        set.push(rt::spawn(run(
            warmup,
            duration,
            notready.clone(),
            notdone.clone(),
            lambda.clone(),
        )));
    }
    for (x, y) in set.into_iter().zip(output.iter()) {
        y.store(x.await, Ordering::SeqCst);
    }
    kernel::profile::end();
    // profile();
}

const TIMESLICE: Duration = Duration::from_millis(50);

#[kernel::core_local]
static mut LAST_YIELD: Duration = Duration::from_millis(0);

async fn maybe_yield() {
    unsafe {
        let now = kvmclock::time_since_boot();
        if now - *LAST_YIELD > TIMESLICE {
            *LAST_YIELD = now;
            rt::yield_now().await;
        }
    }
}

async fn run(
    warmup: Duration,
    duration: Duration,
    notready: Arc<AtomicUsize>,
    notdone: Arc<AtomicUsize>,
    lambda: Lambda,
) -> usize {
    let once = async || {
        let lambda = core::hint::black_box(lambda.clone());
        let thunk = lambda.apply(Value::Tree(vec![Value::Word(1), Value::Word(2)].into()));
        core::hint::black_box(thunk.run());
        maybe_yield().await;
    };

    let warmup_start = kvmclock::time_since_boot();
    while kvmclock::time_since_boot() - warmup_start < warmup {
        once().await;
    }
    notready.fetch_sub(1, Ordering::Release);
    while notready.load(Ordering::Acquire) != 0 {
        once().await;
    }
    let start = kvmclock::time_since_boot();
    let mut iters = 0;
    loop {
        once().await;
        if kvmclock::time_since_boot() - start < duration {
            iters += 1;
        } else {
            break;
        }
    }
    notdone.fetch_sub(1, Ordering::Release);
    while notready.load(Ordering::Acquire) != 0 {
        once().await;
    }
    iters
}

#[allow(unused)]
fn profile() {
    log::info!("--- MOST FREQUENT FUNCTIONS ---");
    let entries = kernel::profile::entries();
    let entries = entries
        .into_iter()
        .map(|(p, x)| {
            if p.is_null() {
                (("USER CODE".into(), 0), x)
            } else {
                (
                    kernel::host::symname(p).unwrap_or_else(|| ("???".into(), 0)),
                    x,
                )
            }
        })
        .fold(BTreeMap::new(), |mut map, ((name, _offset), count)| {
            map.entry(name).and_modify(|e| *e += count).or_insert(count);
            map
        });
    let mut entries = Vec::from_iter(entries);
    entries.sort_by_key(|(_name, count)| *count);
    entries.reverse();
    let total: usize = entries.iter().map(|(_, count)| count).sum();
    for (i, &(ref name, count)) in entries.iter().take(8).enumerate() {
        log::info!(
            "\t{i}: {count:6} ({:3.2}%)- {name}",
            count as f64 / total as f64 * 100.
        );
    }
    log::info!("-------------------------------");
}
