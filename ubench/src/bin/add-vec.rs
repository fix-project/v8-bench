#![no_std]
#![no_main]
#![allow(clippy::needless_range_loop)]

extern crate user;

#[unsafe(no_mangle)]
pub static mut MEMORY: [u32; (1 << 16) / 4] = [0; (1 << 16) / 4];

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        user::syscall::resize(2);

        user::syscall::prompt(0);
        user::syscall::read_tree_unchecked(0, &[0, 1]);

        let mut x: u64 = 0;
        let mut y: u64 = 0;
        user::syscall::read_word_unchecked(0, &mut x);
        user::syscall::read_word_unchecked(1, &mut y);

        for i in 0..4096 {
            MEMORY[i] = x as u32;
        }
        for i in 0..4096 {
            MEMORY[4096 + i] = y as u32;
        }
        for i in 0..4096 {
            MEMORY[8192 + i] = MEMORY[i] + MEMORY[4096 + i];
        }

        let z = MEMORY[8192] as u64;

        user::syscall::create_word(0, z);
        user::syscall::exit(0);
    }
}
