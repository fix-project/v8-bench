#![no_std]
#![no_main]

extern crate user;

const DIM: usize = 64;
static mut SIZE: usize = 0;
static mut MEMORY: [u8; 65536] = [0; 65536];

unsafe fn set(idx: usize, x: usize, y: usize, val: u32) {
    unsafe {
        let base = SIZE*idx;
        let offset = DIM*y + x;
        let address = &mut MEMORY[(base + offset)*4];
        let p: &mut u32 = core::mem::transmute(address);
        *p = val;
    }
}

unsafe fn get(idx: usize, x: usize, y: usize) -> u32 {
    unsafe {
        let base = SIZE*idx;
        let offset = DIM*y + x;
        let address = &mut MEMORY[(base + offset)*4];
        let p: &mut u32 = core::mem::transmute(address);
        *p
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        SIZE = DIM * DIM;
        user::syscall::resize(2);

        user::syscall::prompt(0);
        user::syscall::read_tree_unchecked(0, &[0, 1]);

        let mut lhs: u64 = 0;
        let mut rhs: u64 = 0;
        user::syscall::read_word_unchecked(0, &mut lhs);
        user::syscall::read_word_unchecked(1, &mut rhs);

        let lhs = lhs as u32;
        let rhs = rhs as u32;

        for y in 0..DIM {
            for x in 0..DIM {
                set(0, x, y, lhs);
            }
        }

        for y in 0..DIM {
            for x in 0..DIM {
                set(1, x, y, rhs);
            }
        }

        for y in 0..DIM {
            for x in 0..DIM {
                let mut sum = 0;
                for i in 0..DIM {
                    sum += get(0, i, y) + get(1, x, i);
                }
                set(2, x, y, sum);
            }
        }

        let mut sum = 0;
        for y in 0..DIM {
            for x in 0..DIM {
                sum += get(2, x, y);
            }
        }

        user::syscall::create_word(0, sum as u64);
        user::syscall::exit(0);
    }
}
