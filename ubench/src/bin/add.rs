#![no_std]
#![no_main]

extern crate user;

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

        let z = x as u32 + y as u32;

        user::syscall::create_word(0, z as u64);
        user::syscall::exit(0);
    }
}
