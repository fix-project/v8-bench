#![no_std]
#![no_main]
#![feature(int_roundings)]
extern crate alloc;

use core::alloc::GlobalAlloc;

use alloc::{vec, vec::Vec};
use zune_jpeg::{JpegDecoder, zune_core::colorspace::ColorSpace};

static IMAGE: &[u8] = include_bytes!("../../AS11-36-5339_lrg.jpg");

static mut MEMORY_MAPPED_END: *mut u8 = core::ptr::null_mut();
static mut MEMORY_ALLOCATED_END: *mut u8 = core::ptr::null_mut();

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
                core::arch::wasm32::memory_grow(0, pages);
                MEMORY_MAPPED_END = MEMORY_MAPPED_END.byte_add(pages * 65536);
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

#[unsafe(no_mangle)]
pub extern "C" fn add(_: u32, _: u32) -> u32 {
    unsafe {
        MEMORY_MAPPED_END = (core::arch::wasm32::memory_size(0) * 65536) as *mut u8;
        MEMORY_ALLOCATED_END = (core::arch::wasm32::memory_size(0) * 65536) as *mut u8;
    }

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
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
