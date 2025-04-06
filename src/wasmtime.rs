use crate::SimpleRuntime;
use anyhow::Result;
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

pub struct WasmtimeBenchmark {
    engine: Engine,
    module: Module,
}

impl WasmtimeBenchmark {
    /// # Safety
    ///
    /// This module must expose a function named "add" which takes two i32s and returns an i32.
    pub unsafe fn new(wat: &[u8]) -> Result<Self> {
        let wasm = if &wat[..4] == b"\0asm" {
            wat
        } else {
            &wabt::wat2wasm(wat)?
        };
        let engine = Engine::default();
        let module = Module::new(&engine, wasm)?;

        Ok(WasmtimeBenchmark { engine, module })
    }
}

impl SimpleRuntime for WasmtimeBenchmark {
    type State = Linker<()>;

    fn setup(&self) -> Self::State {
        Linker::new(&self.engine)
    }

    fn iterate(&self, state: &mut Self::State) {
        let mut store: Store<()> = Store::new(&self.engine, ());
        let instance = state.instantiate(&mut store, &self.module).unwrap();
        let add: TypedFunc<(u32, u32), u32> = instance.get_typed_func(&mut store, "add").unwrap();
        add.call(&mut store, (1, 2)).unwrap();
    }
}
