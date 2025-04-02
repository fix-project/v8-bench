#![no_std]
#![no_main]

extern crate user;

#[unsafe(no_mangle)]
pub static mut MEMORY: [u32; (1 << 16) / 4] = [0; (1 << 16) / 4];

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        user::syscall::resize(2);

        user::syscall::prompt(0);
        MEMORY[0] = 1;
        user::syscall::read_tree_unchecked(0, &[0, 1]);

        let mut x: u64 = 0;
        let mut y: u64 = 0;
        user::syscall::read_word_unchecked(0, &mut x);
        user::syscall::read_word_unchecked(1, &mut y);

        MEMORY[0] = x as u32;
        MEMORY[1] = y as u32;
        MEMORY[2] = MEMORY[0] + MEMORY[1];
        let z = MEMORY[2] as u64;

        user::syscall::create_word(0, z);
        user::syscall::exit(0);
    }
}
