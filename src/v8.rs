use std::{
    marker::PhantomData,
    path::PathBuf,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use v8::{CompiledWasmModule, Local, Object, WasmModuleObject};

use crate::Benchmark;

static ONE_TIME_INIT: LazyLock<()> = LazyLock::new(|| {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::set_flags_from_command_line(vec!["--liftoff".into(), "--no-wasm-tier-up".into()]);
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
});

fn compile(module: &[u8]) -> CompiledWasmModule {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    v8::WasmModuleObject::compile(scope, module)
        .expect("could not compile wasm")
        .get_compiled_module()
}

pub trait V8Mode {}
pub struct SameIsolateSameContext;
impl V8Mode for SameIsolateSameContext {}
pub struct SameIsolateNewContext;
impl V8Mode for SameIsolateNewContext {}
pub struct NewIsolate;
impl V8Mode for NewIsolate {}

pub struct V8Benchmark<MODE: V8Mode> {
    module: CompiledWasmModule,
    _phantom: PhantomData<MODE>,
}

impl<MODE: V8Mode> V8Benchmark<MODE> {
    pub fn new(wat: PathBuf) -> Result<Self> {
        LazyLock::force(&ONE_TIME_INIT);
        let data = std::fs::read(wat)?;
        Ok(V8Benchmark {
            module: compile(&wabt::wat2wasm(&data)?),
            _phantom: PhantomData,
        })
    }
}

impl V8Benchmark<NewIsolate> {
    pub fn iterate(&self, todo: usize) {
        for _ in 0..todo {
            let isolate = &mut v8::Isolate::new(Default::default());
            let mut handle_scope = v8::HandleScope::new(isolate);
            let context = v8::Context::new(&mut handle_scope, Default::default());
            let global = context.global(&mut handle_scope);
            let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
            let module =
                v8::WasmModuleObject::from_compiled_module(&mut context_scope, &self.module)
                    .unwrap();
            body(global, &mut context_scope, module);
        }
    }
}

impl Benchmark for V8Benchmark<NewIsolate> {
    fn run(&self, parallel: usize, iterations_per_thread: usize) -> Duration {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.iterate(iterations_per_thread);
                });
                handles.push(handle);
            }
            let start = Instant::now();
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            for h in handles {
                h.join().unwrap();
            }
            start.elapsed()
        })
    }
}

impl V8Benchmark<SameIsolateNewContext> {
    pub fn iterate(&self, todo: usize) {
        let isolate = &mut v8::Isolate::new(Default::default());
        for _ in 0..todo {
            let mut handle_scope = v8::HandleScope::new(isolate);
            let context = v8::Context::new(&mut handle_scope, Default::default());
            let global = context.global(&mut handle_scope);
            let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
            let module =
                v8::WasmModuleObject::from_compiled_module(&mut context_scope, &self.module)
                    .unwrap();
            body(global, &mut context_scope, module);
        }
    }
}

impl Benchmark for V8Benchmark<SameIsolateNewContext> {
    fn run(&self, parallel: usize, iterations_per_thread: usize) -> Duration {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.iterate(iterations_per_thread);
                });
                handles.push(handle);
            }
            let start = Instant::now();
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            for h in handles {
                h.join().unwrap();
            }
            start.elapsed()
        })
    }
}

impl V8Benchmark<SameIsolateSameContext> {
    pub fn iterate(&self, todo: usize) {
        let isolate = &mut v8::Isolate::new(Default::default());
        let mut handle_scope = v8::HandleScope::new(isolate);
        let context = v8::Context::new(&mut handle_scope, Default::default());
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
        let module =
            v8::WasmModuleObject::from_compiled_module(&mut context_scope, &self.module).unwrap();
        core::mem::drop(context_scope);
        for _ in 0..todo {
            let mut handle_scope = v8::HandleScope::new(&mut handle_scope);
            let global = context.global(&mut handle_scope);
            let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
            body(global, &mut context_scope, module);
        }
    }
}

impl Benchmark for V8Benchmark<SameIsolateSameContext> {
    fn run(&self, parallel: usize, iterations_per_thread: usize) -> Duration {
        let begin = AtomicBool::new(false);
        std::thread::scope(|s| {
            let mut handles = vec![];
            for _ in 0..parallel {
                let handle = s.spawn(|| {
                    while !begin.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    self.iterate(iterations_per_thread);
                });
                handles.push(handle);
            }
            let start = Instant::now();
            begin.store(true, Ordering::SeqCst);
            for h in handles.iter() {
                h.thread().unpark();
            }
            for h in handles {
                h.join().unwrap();
            }
            start.elapsed()
        })
    }
}

fn body(
    global: Local<Object>,
    scope: &mut v8::HandleScope,
    module: Local<WasmModuleObject>,
) -> u32 {
    let webassembly = v8::String::new(scope, "WebAssembly").unwrap().into();
    let instance = v8::String::new(scope, "Instance").unwrap().into();
    let exports = v8::String::new(scope, "exports").unwrap().into();
    let add = v8::String::new(scope, "add").unwrap().into();
    let x = v8::Number::new(scope, 1.);
    let y = v8::Number::new(scope, 2.);
    let webassembly = global
        .get(scope, webassembly)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let instance = webassembly
        .get(scope, instance)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let instance = instance.cast::<v8::Function>();
    let instance = instance
        .new_instance(scope, &[module.into()])
        .unwrap()
        .to_object(scope)
        .unwrap();
    let exports = instance
        .get(scope, exports)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let add = exports
        .get(scope, add)
        .unwrap()
        .to_object(scope)
        .unwrap()
        .cast::<v8::Function>();
    let result = add
        .call(scope, global.into(), &[x.into(), y.into()])
        .unwrap();
    let result = result.to_uint32(scope).unwrap();
    result.value()
}
