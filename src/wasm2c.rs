use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use crate::SingleThreadedRuntime;
use anyhow::Result;
use include_directory::{Dir, include_directory};

static WASM2C_RT: Dir<'_> = include_directory!("$CARGO_MANIFEST_DIR/wasm2c");

pub struct Wasm2CBenchmark {
    lib: libloading::Library,
}

impl Wasm2CBenchmark {
    /// # Safety
    ///
    /// This module must expose a function named "add" which takes two i32s and returns an i32.
    pub unsafe fn new(wat: PathBuf, hardware: bool) -> Result<Self> {
        let data = std::fs::read(wat)?;
        let wasm = wabt::wat2wasm(data)?;
        let temp_dir = tempfile::tempdir()?;
        let mut wasm_file = temp_dir.path().to_path_buf();
        wasm_file.push("module.wasm");
        std::fs::write(&wasm_file, &wasm)?;
        let mut c_file = temp_dir.path().to_path_buf();
        c_file.push("module.c");

        // Using wasm2c 1.0.34 from the Ubuntu repos
        let wasm2c = Command::new("wasm2c")
            .args([
                "-o",
                c_file.to_str().unwrap(),
                "-n",
                "module",
                wasm_file.to_str().unwrap(),
            ])
            .status()?;
        assert!(wasm2c.success());
        WASM2C_RT.extract(&temp_dir)?;

        let mut so_file = temp_dir.path().to_path_buf();
        so_file.push("module.so");

        let mut lib = temp_dir.path().to_path_buf();
        lib.push("lib.c");

        let mut wasm_rt_impl = temp_dir.path().to_path_buf();
        wasm_rt_impl.push("wasm-rt-impl.c");

        let flags = if hardware {
            vec![
                "-DWASM_RT_USE_MMAP=1",
                "-DWASM_RT_MEMCHECK_GUARD_PAGES=1",
                "-DWASM_RT_MEMCHECK_BOUNDS_CHECK=0",
            ]
        } else {
            vec![
                "-DWASM_RT_USE_MMAP=0",
                "-DWASM_RT_MEMCHECK_GUARD_PAGES=0",
                "-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1",
            ]
        };

        let cc = Command::new("cc")
            .args([
                "-o",
                so_file.to_str().unwrap(),
                "-I",
                temp_dir.path().to_str().unwrap(),
                c_file.to_str().unwrap(),
                lib.to_str().unwrap(),
                wasm_rt_impl.to_str().unwrap(),
                "-lm",
                "-fPIC",
                "-shared",
                "-O2",
                "-fno-optimize-sibling-calls",
                "-frounding-math",
                "-fsignaling-nans",
            ])
            .args(flags)
            .status()?;
        assert!(cc.success());

        unsafe {
            let lib = libloading::Library::new(so_file)?;
            let wasm_rt_init: libloading::Symbol<unsafe extern "C" fn()> =
                lib.get(b"wasm_rt_init")?;
            wasm_rt_init();
            Ok(Wasm2CBenchmark { lib })
        }
    }
}

impl Drop for Wasm2CBenchmark {
    fn drop(&mut self) {
        unsafe {
            let wasm_rt_free: libloading::Symbol<unsafe extern "C" fn()> =
                self.lib.get(b"wasm_rt_free").unwrap();
            wasm_rt_free();
        }
    }
}

impl SingleThreadedRuntime for Wasm2CBenchmark {
    fn run(&self, duration: Duration) -> usize {
        unsafe {
            let allocate_module: libloading::Symbol<
                unsafe extern "C" fn() -> *mut std::ffi::c_void,
            > = self.lib.get(b"allocate_module").unwrap();
            let add: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, u32, u32)> =
                self.lib.get(b"w2c_module_add").unwrap();
            let free_module: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void)> =
                self.lib.get(b"free_module").unwrap();
            let start = Instant::now();
            let mut i = 0;
            while start.elapsed() < duration {
                let module = allocate_module();
                add(module, 1, 2);
                free_module(module);
                i += 1;
            }
            i
        }
    }
}
