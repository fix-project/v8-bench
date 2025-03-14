#include "v8env.hh"

#include "include/libplatform/libplatform.h"
#include "include/v8-context.h"
#include "v8-memory-span.h"
#include "v8-platform.h"

using namespace std;
using namespace v8;

V8Env::V8Env(char *argv0, bool bounds_checks) {
  // Initialize V8.
  V8::InitializeICUDefaultLocation(argv0);
  V8::InitializeExternalStartupData(argv0);
  platform_ = platform::NewDefaultPlatform();
  // TurboFan Compiler Only
  if (bounds_checks) {
    V8::SetFlagsFromString(
        "--no-liftoff --no-wasm-tier-up --wasm-enforce-bounds-checks");
  } else {
    V8::SetFlagsFromString("--no-liftoff --no-wasm-tier-up");
  }
  V8::InitializePlatform(platform_.get());
  V8::Initialize();

  create_params_.array_buffer_allocator =
      ArrayBuffer::Allocator::NewDefaultAllocator();
}

const Isolate::CreateParams &V8Env::get_create_params() const {
  return create_params_;
}

const CompiledWasmModule &V8Env::get_compiled_wasm() const {
  return wasm_compiled_.value();
}
const CompiledWasmModule &V8Env::compile(span<uint8_t> bin) {
  Isolate *isolate = Isolate::New(create_params_);
  {
    Isolate::Scope isolate_scope(isolate);
    HandleScope handle_scope(isolate);
    Local<Context> context = Context::New(isolate);
    Context::Scope context_scope(context);

    Local<WasmModuleObject> module =
        WasmModuleObject::Compile(
            isolate, MemorySpan<const uint8_t>(bin.data(), bin.size()))
            .ToLocalChecked();

    wasm_compiled_.emplace(module->GetCompiledModule());
  }
  isolate->Dispose();

  return wasm_compiled_.value();
}

V8Env::~V8Env() {
  wasm_compiled_.reset();
  V8::Dispose();
  V8::DisposePlatform();
  delete create_params_.array_buffer_allocator;
}
