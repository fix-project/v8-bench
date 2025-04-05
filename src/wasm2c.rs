use std::{mem::MaybeUninit, process::Command, sync::Arc};

use crate::SimpleRuntime;
use anyhow::Result;
use include_directory::{Dir, include_directory};
use ouroboros::self_referencing;

static WASM2C_RT: Dir<'_> = include_directory!("$CARGO_MANIFEST_DIR/wasm2c");

pub struct Wasm2CBenchmark {
    lib: Arc<libloading::Library>,
}

impl Wasm2CBenchmark {
    /// # Safety
    ///
    /// This module must expose a function named "add" which takes two i32s and returns an i32.
    pub unsafe fn new(wat: &[u8], hardware: bool) -> Result<Self> {
        let wasm = if &wat[..4] == b"\0asm" {
            wat
        } else {
            &wabt::wat2wasm(wat)?
        };
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
            Ok(Wasm2CBenchmark { lib: Arc::new(lib) })
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

#[self_referencing]
pub struct State {
    library: Arc<libloading::Library>,
    memory: Box<[MaybeUninit<u8>]>,
    #[borrows(library)]
    #[covariant]
    add: libloading::Symbol<'this, unsafe extern "C" fn(*mut std::ffi::c_void, u32, u32)>,
    #[borrows(library)]
    #[covariant]
    instantiate: libloading::Symbol<'this, unsafe extern "C" fn(*mut std::ffi::c_void)>,
    #[borrows(library)]
    #[covariant]
    free: libloading::Symbol<'this, unsafe extern "C" fn(*mut std::ffi::c_void)>,
}

impl SimpleRuntime for Wasm2CBenchmark {
    type State = State;

    fn setup(&self) -> Self::State {
        let library = self.lib.clone();
        unsafe {
            let module_size: libloading::Symbol<unsafe extern "C" fn() -> usize> =
                library.get(b"module_size").unwrap();
            let size = module_size();
            let memory = Box::new_uninit_slice(size);
            StateBuilder {
                library,
                memory,
                add_builder: |lib| lib.get(b"w2c_module_add").unwrap(),
                instantiate_builder: |lib| lib.get(b"wasm2c_module_instantiate").unwrap(),
                free_builder: |lib| lib.get(b"wasm2c_module_free").unwrap(),
            }
            .build()
        }
    }

    fn iterate(&self, state: &mut Self::State) {
        state.with_mut(|fields| {
            let instantiate = fields.instantiate;
            let add = fields.add;
            let free = fields.free;
            let module: *mut MaybeUninit<u8> = fields.memory.as_mut_ptr();
            let module = module as *mut std::ffi::c_void;
            unsafe {
                instantiate(module);
                add(module, 1, 2);
                free(module);
            }
        })
    }
}
