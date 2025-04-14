use crate::SimpleRuntime;
use anyhow::Result;
use clone3::{Clone3, CloneArgs};
use libc::{__WALL, __WNOTHREAD, P_ALL, P_PID, WEXITED, siginfo_t, waitid};
use libc::{
    CLONE_CHILD_CLEARTID, CLONE_NEWCGROUP, CLONE_NEWIPC, CLONE_NEWNET, CLONE_NEWNS, CLONE_NEWPID,
    CLONE_NEWUSER, CLONE_NEWUTS, CLONE_PARENT_SETTID, CLONE_VM, EAGAIN, FUTEX_BITSET_MATCH_ANY,
    FUTEX_WAIT, SA_NOCLDWAIT, SIGCHLD, SYS_exit, SYS_futex, sigaction, syscall,
};
use libc::{
    MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, MAP_STACK, PROT_READ, PROT_WRITE, mmap, munmap,
};
use libc::{chdir, chroot, clearenv, clone};
use std::{
    arch::naked_asm,
    ffi::{c_char, c_int, c_void},
    fs::{File, create_dir, create_dir_all, exists},
    io::Error,
    path::Path,
    ptr, slice,
};

use zune_jpeg::{JpegDecoder, zune_core::colorspace::ColorSpace};
static IMAGE: &[u8] = include_bytes!("../AS11-36-5339_lrg.jpg");
static CGROUP_DIR: &str = "/sys/fs/cgroup/";
static ROOT_DIR: &str = concat!(env!("HOME"), "/cgroup-bench/root\0");
static ROOT: &str = concat!("/", "\0");

#[repr(C)]
struct Arg(i32, i32);

struct SyscallReturnCode(c_int);

impl SyscallReturnCode {
    fn into_result(self) -> std::io::Result<()> {
        if self.0 == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn into_result_value(self) -> std::io::Result<c_int> {
        if self.0 == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(self.0)
        }
    }
}

pub enum CloneBenchmarkType {
    Add,
    AddVec,
    MatMul64,
    MatMul128,
    Jpeg,
}

static mut DIM: usize = 64;
static mut SIZE: usize = 0;
static mut MEMORY: [u8; 3 * 65536] = [0; 3 * 65536];

unsafe fn set(idx: usize, x: usize, y: usize, val: u32) {
    unsafe {
        let base = SIZE * idx;
        let offset = DIM * y + x;
        let address = &mut MEMORY[(base + offset) * 4];
        let p: &mut u32 = core::mem::transmute(address);
        *p = val;
    }
}

unsafe fn get(idx: usize, x: usize, y: usize) -> u32 {
    unsafe {
        let base = SIZE * idx;
        let offset = DIM * y + x;
        let address = &mut MEMORY[(base + offset) * 4];
        let p: &mut u32 = core::mem::transmute(address);
        *p
    }
}

extern "C" fn matmul(arg: *mut c_void, dim: usize) -> c_int {
    let arg: &mut Arg = unsafe { &mut *(arg as *mut Arg) };
    let lhs = arg.0 as u32;
    let rhs = arg.1 as u32;

    unsafe {
        DIM = dim;
        SIZE = DIM * DIM;

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
        sum.try_into().unwrap()
    }
}

extern "C" fn matmul64(arg: *mut c_void) -> c_int {
    matmul(arg, 64)
}

extern "C" fn matmul128(arg: *mut c_void) -> c_int {
    matmul(arg, 128)
}

extern "C" fn add(arg: *mut c_void) -> c_int {
    let arg: &mut Arg = unsafe { &mut *(arg as *mut Arg) };
    arg.0 + arg.1
}

extern "C" fn addvec(arg: *mut c_void) -> c_int {
    let arg: &mut Arg = unsafe { &mut *(arg as *mut Arg) };
    let x = arg.0;
    let y = arg.1;

    let mem: &mut [u32; 65536 >> 2] = unsafe { &mut *(&raw mut MEMORY as *mut [u32; 16384]) };

    for i in 0..4096 {
        mem[i] = x as u32;
    }
    for i in 0..4096 {
        mem[4096 + i] = y as u32;
    }
    for i in 0..4096 {
        mem[8192 + i] = mem[i] + mem[4096 + i];
    }

    let z = mem[8192];
    z.try_into().unwrap()
}

extern "C" fn jpeg(_: *mut c_void) -> c_int {
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
    0
}

fn chenv() -> std::io::Result<()> {
    SyscallReturnCode(unsafe { clearenv() }).into_result()?;
    SyscallReturnCode(unsafe { chroot(ROOT_DIR.as_ptr() as *const c_char) }).into_result()?;
    SyscallReturnCode(unsafe { chdir(ROOT.as_ptr() as *const c_char) }).into_result()?;
    Ok(())
}

extern "C" fn add_containered(arg: *mut c_void) -> c_int {
    match chenv() {
        Ok(_) => add(arg),
        Err(e) => e.raw_os_error().unwrap(),
    }
}

extern "C" fn addvec_containered(arg: *mut c_void) -> c_int {
    match chenv() {
        Ok(_) => addvec(arg),
        Err(e) => e.raw_os_error().unwrap(),
    }
}

extern "C" fn matmul64_containered(arg: *mut c_void) -> c_int {
    match chenv() {
        Ok(_) => matmul64(arg),
        Err(e) => e.raw_os_error().unwrap(),
    }
}

extern "C" fn matmul128_containered(arg: *mut c_void) -> c_int {
    match chenv() {
        Ok(_) => matmul128(arg),
        Err(e) => e.raw_os_error().unwrap(),
    }
}

extern "C" fn jpeg_containered(arg: *mut c_void) -> c_int {
    match chenv() {
        Ok(_) => jpeg(arg),
        Err(e) => e.raw_os_error().unwrap(),
    }
}

pub struct CloneBenchmark {
    set_flags: bool,
    create_process: bool,
    clone_into_cgroup: bool,
    clone3: bool,
    cb: extern "C" fn(*mut c_void) -> c_int,
}

#[repr(C)]
#[repr(align(16))]
struct StackHead {
    thread_entry: usize,
    cb: extern "C" fn(*mut c_void) -> c_int,
    arg: *mut c_void,
}

extern "C" fn clone3_threadenv(stackhead: *const StackHead) {
    let stack: &StackHead = unsafe { &(*stackhead) };
    let res = (stack.cb)(stack.arg);
    unsafe {
        syscall(SYS_exit, res as u64);
    }
}

#[naked]
unsafe fn clone3_newthread(stack: *const CloneArgs) -> i64 {
    unsafe {
        naked_asm!(
            "mov  rsi, 88",  // arg2 = sizeof(CloneArgs)
            "mov  rdi, rdi", // arg1 = CloneArgs*
            "mov  eax, 435", // SYS_clone3
            "syscall",
            "mov  rdi, rsp", // entry point argument
            "ret",
        );
    }
}

struct MMapStack {
    addr: *mut c_void,
    size: usize,
}

impl MMapStack {
    fn new(size: usize) -> Self {
        let addr = unsafe {
            mmap(
                ptr::null_mut::<c_void>(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK,
                -1,
                0,
            )
        };
        if addr == MAP_FAILED {
            panic!("Failed to mmap");
        }

        MMapStack { addr, size }
    }

    fn as_slice(&self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.addr as *mut u8, self.size) }
    }
}

impl Drop for MMapStack {
    fn drop(&mut self) {
        unsafe {
            munmap(self.addr, self.size);
        }
    }
}

pub struct CloneBenchmarkState {
    cgroup: Option<File>,
    stack: Option<MMapStack>,
    arg: Box<Arg>,
    child_pid: i32,
}

impl CloneBenchmark {
    fn clone_helper(&self, state: &mut CloneBenchmarkState) -> c_int {
        if self.clone3 {
            let mut config = Clone3::default();

            if self.set_flags {
                config.flag_newcgroup();
                config.flag_newipc();
                config.flag_newnet();
                config.flag_newns();
                config.flag_newpid();
                config.flag_newuts();
                config.flag_newuser();
            }

            if let Some(cgroup) = state.cgroup.as_ref() {
                config.flag_into_cgroup(cgroup);
            }

            if !self.create_process {
                // Set exit signal
                config.exit_signal(SIGCHLD.try_into().unwrap());

                // Set up futex
                let child_pid_ptr: *mut i32 = &mut state.child_pid;
                config.flag_parent_settid(unsafe { &mut *child_pid_ptr });
                config.flag_child_cleartid(unsafe { &mut *child_pid_ptr });

                let stack_head = StackHead {
                    thread_entry: clone3_threadenv as usize,
                    cb: self.cb,
                    arg: Box::as_mut_ptr(&mut state.arg) as *mut c_void,
                };

                let stack_head_ptr = unsafe {
                    &mut *(state
                        .stack
                        .as_ref()
                        .unwrap()
                        .as_slice()
                        .as_mut_ptr_range()
                        .end as *mut StackHead)
                        .offset(-1)
                };
                // Load stack_head to the beginning of allocated stack
                *stack_head_ptr = stack_head;

                config.flag_vm(state.stack.as_ref().unwrap().as_slice());
            }

            if self.create_process {
                match unsafe { config.call() } {
                    Ok(0) => unsafe {
                        libc::exit((self.cb)(Box::as_mut_ptr(&mut state.arg) as *mut c_void))
                    },
                    Ok(res) => res,
                    Err(errno) => errno.0,
                }
            } else {
                let mut args = config.as_clone_args();
                // Exclude stack_head from the stack
                args.stack_size -= core::mem::size_of::<StackHead>() as u64;
                unsafe { clone3_newthread(&args).try_into().unwrap() }
            }
        } else {
            let mut flags: c_int = CLONE_VM | CLONE_PARENT_SETTID | CLONE_CHILD_CLEARTID;

            if self.set_flags {
                flags = flags
                    | CLONE_NEWCGROUP
                    | CLONE_NEWIPC
                    | CLONE_NEWNET
                    | CLONE_NEWNS
                    | CLONE_NEWPID
                    | CLONE_NEWUTS
                    | CLONE_NEWUSER;
            }

            flags |= SIGCHLD;
            unsafe {
                clone(
                    self.cb,
                    state
                        .stack
                        .as_ref()
                        .unwrap()
                        .as_slice()
                        .as_mut_ptr_range()
                        .end as *mut c_void,
                    flags,
                    Box::as_mut_ptr(&mut state.arg) as *mut c_void,
                    (&mut state.child_pid) as *mut i32,
                    ptr::null::<c_void>(),
                    (&mut state.child_pid) as *mut i32,
                )
            }
        }
    }

    pub fn new(
        benchmark: CloneBenchmarkType,
        create_process: bool,
        set_flags: bool,
        chenv: bool,
        clone3: bool,
        clone_into_cgroup: bool,
    ) -> Result<Self> {
        Ok(CloneBenchmark {
            set_flags,
            create_process,
            clone3,
            clone_into_cgroup: clone3 && clone_into_cgroup,
            cb: match benchmark {
                CloneBenchmarkType::Add => {
                    if chenv {
                        add_containered
                    } else {
                        add
                    }
                }
                CloneBenchmarkType::AddVec => {
                    if chenv {
                        addvec_containered
                    } else {
                        addvec
                    }
                }
                CloneBenchmarkType::MatMul64 => {
                    if chenv {
                        matmul64_containered
                    } else {
                        matmul64
                    }
                }
                CloneBenchmarkType::MatMul128 => {
                    if chenv {
                        matmul128_containered
                    } else {
                        matmul128
                    }
                }
                CloneBenchmarkType::Jpeg => {
                    if chenv {
                        jpeg_containered
                    } else {
                        jpeg
                    }
                }
            },
        })
    }
}

impl SimpleRuntime for CloneBenchmark {
    type State = CloneBenchmarkState;

    fn setup(&self) -> CloneBenchmarkState {
        if !self.create_process {
            // Terminated child does not become zombie
            let mut act: sigaction = unsafe { std::mem::zeroed() };
            act.sa_flags = SA_NOCLDWAIT;
            if let Err(error) = SyscallReturnCode(unsafe {
                sigaction(
                    SIGCHLD,
                    &act as *const sigaction,
                    ptr::null_mut::<sigaction>(),
                )
            })
            .into_result()
            {
                panic!("Failed to set SA_NOCLDWAIT: {}", error)
            }
        }

        let root_path = Path::new(ROOT_DIR.trim_matches('\0'));
        match create_dir_all(root_path) {
            Ok(_) => (),
            Err(error) => {
                if !exists(root_path).unwrap() {
                    panic!("{}", error)
                }
            }
        }

        let cgroup: Option<File> = if self.clone_into_cgroup {
            let path = Path::new(CGROUP_DIR).join("cg1");

            match create_dir(&path) {
                Ok(_) => (),
                Err(error) => {
                    if !exists(&path).unwrap() {
                        panic!("{}", error)
                    }
                }
            }
            Some(File::open(&path).expect("Failed to open cgroup"))
        } else {
            None
        };

        CloneBenchmarkState {
            cgroup,
            stack: if self.create_process {
                None
            } else {
                Some(MMapStack::new(1 << 20))
            },
            arg: Box::new(Arg(42, 8)),
            child_pid: 0,
        }
    }

    fn iterate(&self, state: &mut Self::State) {
        let res = SyscallReturnCode(self.clone_helper(state))
            .into_result_value()
            .expect("Failed to clone.");
        let res = if self.create_process {
            let mut info: siginfo_t = unsafe { std::mem::zeroed() };
            SyscallReturnCode(unsafe {
                waitid(
                    P_PID,
                    res.try_into().unwrap(),
                    &mut info,
                    __WALL | __WNOTHREAD | WEXITED,
                )
            })
        } else {
            SyscallReturnCode(unsafe {
                syscall(
                    SYS_futex,
                    (&mut state.child_pid) as *mut i32,
                    FUTEX_WAIT,
                    res,
                    0,
                    FUTEX_BITSET_MATCH_ANY,
                )
                .try_into()
                .unwrap()
            })
        }
        .into_result();

        if let Err(error) = res {
            if let Some(EAGAIN) = error.raw_os_error() {
            } else {
                panic!("futex_wait error: {}", error)
            }
        }
    }
}
