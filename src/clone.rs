use crate::SingleThreadedRuntime;
use anyhow::Result;
use clone3::Clone3;
use libc::{__WALL, __WNOTHREAD, P_PID, WEXITED};
use libc::{
    CLONE_NEWCGROUP, CLONE_NEWIPC, CLONE_NEWNET, CLONE_NEWNS, CLONE_NEWPID, CLONE_NEWUSER,
    CLONE_NEWUTS,
};
use libc::{chdir, chroot, clearenv, clone, exit, siginfo_t, waitid};
use std::{
    ffi::{c_char, c_int, c_void},
    fs::{File, create_dir, exists, remove_dir},
    io::Error,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
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

extern "C" fn add(arg: *mut c_void) -> c_int {
    let arg: &mut Arg = unsafe { &mut *(arg as *mut Arg) };
    arg.0 + arg.1
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

pub struct CloneBenchmark {
    set_flags: bool,
    clone_into_cgroup: bool,
    clone3: bool,
    cb: extern "C" fn(*mut c_void) -> c_int,
}

impl CloneBenchmark {
    fn full_flags(config: &mut Clone3) {
        config.flag_newcgroup();
        config.flag_newipc();
        config.flag_newnet();
        config.flag_newns();
        config.flag_newpid();
        config.flag_newuts();
        config.flag_newuser();
    }

    unsafe fn clone_helper(
        &self,
        config: &mut Clone3,
        stack: &mut [u8],
        arg_ptr: *mut Arg,
    ) -> Result<c_int, Error> {
        if self.clone3 {
            match unsafe { config.call() } {
                Ok(0) => {
                    unsafe { exit((self.cb)(arg_ptr as *mut c_void)) };
                }
                Ok(child) => Ok(child),
                Err(errno) => Err(Error::from_raw_os_error(errno.0)),
            }
        } else {
            let flags: c_int = if self.set_flags {
                CLONE_NEWCGROUP
                    | CLONE_NEWIPC
                    | CLONE_NEWNET
                    | CLONE_NEWNS
                    | CLONE_NEWPID
                    | CLONE_NEWUTS
                    | CLONE_NEWUSER
            } else {
                0
            };
            SyscallReturnCode(unsafe {
                clone(
                    self.cb,
                    stack.as_mut_ptr_range().end as *mut c_void,
                    flags,
                    arg_ptr as *mut c_void,
                )
            })
            .into_result_value()
        }
    }

    pub fn new(
        set_flags: bool,
        chenv: bool,
        clone3: bool,
        clone_into_cgroup: bool,
    ) -> Result<Self> {
        Ok(CloneBenchmark {
            set_flags,
            clone3,
            clone_into_cgroup: clone3 && clone_into_cgroup,
            cb: if chenv { add_containered } else { add },
        })
    }
}

impl SingleThreadedRuntime for CloneBenchmark {
    fn run(&self, duration: Duration) -> usize {
        let idx = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = Path::new(CGROUP_DIR).join(format!("cg{}", idx));

        let cgroup: Option<File> = if self.clone_into_cgroup {
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

        let mut clone3_config = Clone3::default();
        if self.set_flags {
            CloneBenchmark::full_flags(&mut clone3_config);
        }
        if self.clone_into_cgroup {
            clone3_config.flag_into_cgroup(cgroup.as_ref().unwrap());
        }

        let mut arg = Box::new(Arg(7, 8));
        let arg_ptr: *mut Arg = Box::as_mut_ptr(&mut arg);
        let mut i = 0;
        let mut stack: [u8; 1024] = [0; 1024];

        let start = Instant::now();
        while start.elapsed() < duration {
            let res = unsafe { self.clone_helper(&mut clone3_config, &mut stack, arg_ptr) };
            let res = match res {
                Ok(child) => SyscallReturnCode(unsafe {
                    let mut info: siginfo_t = std::mem::zeroed();
                    waitid(
                        P_PID,
                        child.try_into().unwrap(),
                        &mut info,
                        __WALL | __WNOTHREAD | WEXITED,
                    )
                })
                .into_result(),
                Err(error) => {
                    panic!("clone error: {}", error)
                }
            };

            if let Err(error) = res {
                panic!("waitid error: {}", error)
            }
            i += 1;
        }

        if self.clone_into_cgroup {
            let _ = remove_dir(&path);
        }
        i
    }
}
