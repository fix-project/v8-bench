#![no_std]
#![no_main]
#![feature(int_roundings)]
#![feature(new_zeroed_alloc)]

extern crate alloc;
extern crate user;

use core::alloc::GlobalAlloc;

use alloc::{boxed::Box, vec, vec::Vec};
use zune_jpeg::{JpegDecoder, zune_core::colorspace::ColorSpace};

static IMAGE: &[u8] = include_bytes!("../../../AS11-36-5339_lrg.jpg");

unsafe extern "C" {
    static _end: core::ffi::c_void;
}

static mut MEMORY_MAPPED_END: *mut u8 = core::ptr::null_mut();
static mut MEMORY_ALLOCATED_END: *mut u8 = core::ptr::null_mut();

core::arch::global_asm!(
    "
.globl on_stack
on_stack:
    mov rax, rdi
    mov rbp, 0
    mov rsp, rax
    push rax
    call rsi
    ud2
"
);

unsafe extern "C" {
    pub fn on_stack(stack: *mut u8, f: extern "C" fn() -> !) -> !;
}

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe {
            let mut current = MEMORY_ALLOCATED_END;
            let align = layout.align();
            if current as usize % align != 0 {
                current = current.byte_add(align - (current as usize % align));
            }
            assert_eq!(current as usize % align, 0);
            let start = current;
            let end = start.byte_add(layout.size());
            if end > MEMORY_MAPPED_END {
                let pages = end.byte_offset_from(MEMORY_MAPPED_END).div_ceil(65536) as usize;
                let pages = pages * 65536 / 4096;
                user::syscall::map_new_pages(MEMORY_MAPPED_END as _, pages);
                MEMORY_MAPPED_END = MEMORY_MAPPED_END.byte_add(pages * 4096);
            }
            MEMORY_ALLOCATED_END = end;
            start
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let _ = ptr;
        let _ = layout;
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

fn align(unaligned_val: usize, alignment: usize) -> usize {
    let mask = alignment - 1;
    unaligned_val + (-(unaligned_val as isize) as usize & mask)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    user::syscall::resize(3);

    unsafe {
        // skip initial 4KB stack
        let end = &raw const _end;
        let end = align(end as usize, 4096) as *mut u8;
        MEMORY_MAPPED_END = end.byte_add(4096);
        user::syscall::create_word(2, MEMORY_MAPPED_END as usize as u64);
        MEMORY_ALLOCATED_END = MEMORY_MAPPED_END;
        // Allocate 2MB stack.
        let size = 2 * 1024 * 1024;
        let buffer: Box<[u8]> = Box::new_zeroed_slice(size).assume_init();
        let stack = Box::leak(buffer);
        let end = stack.as_mut_ptr().byte_add(size);
        assert_eq!(end as usize % 16, 0);
        on_stack(end, _finish)
    }
}

pub extern "C" fn _finish() -> ! {
    unsafe {
        user::syscall::prompt(0);
        user::syscall::read_tree_unchecked(0, &[0, 1]);

        // we don't actually need the arguments

        let mut jpeg = JpegDecoder::new(IMAGE);
        let pixels = jpeg.decode().unwrap();
        let (w, h) = jpeg.dimensions().unwrap();
        assert_eq!(jpeg.get_output_colorspace().unwrap(), ColorSpace::RGB);
        let bpp = pixels.len() / (w * h);
        let width = 32;
        let height = 32;
        let x_scale = w / width;
        let y_scale = h / height;
        let mut output: Vec<u8> = vec![0; width * height * bpp];
        for y in 0..height {
            for x in 0..width {
                let out_range = &mut output[(y * width + x) * bpp..];
                let in_range = &pixels[(y * y_scale * w + x * x_scale) * bpp..];
                let out_pixel = &mut out_range[..bpp];
                let in_pixel = &in_range[..bpp];
                out_pixel.copy_from_slice(in_pixel);
            }
        }
        core::hint::black_box(output);

        user::syscall::create_word(0, 0);
        user::syscall::exit(0);
    }
}
