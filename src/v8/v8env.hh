#pragma once

#include "include/libplatform/libplatform.h"
#include "include/v8-context.h"
#include "include/v8-initialization.h"
#include "include/v8-isolate.h"
#include "include/v8-local-handle.h"
#include "include/v8-primitive.h"
#include "include/v8.h"
#include "v8-memory-span.h"
#include "v8-wasm.h"
#include <span>
#include <v8-platform.h>

class V8Env {
private:
  v8::Isolate::CreateParams create_params_;
  std::optional<v8::CompiledWasmModule> wasm_compiled_;
  std::unique_ptr<v8::Platform> platform_;

public:
  V8Env(char *argv[]) {
    // Initialize V8.
    v8::V8::InitializeICUDefaultLocation(argv[0]);
    v8::V8::InitializeExternalStartupData(argv[0]);
    platform_ = v8::platform::NewDefaultPlatform();
    // TurboFan Compiler Only
    v8::V8::SetFlagsFromString("--no-liftoff --no-wasm-tier-up");
    v8::V8::InitializePlatform(platform_.get());
    v8::V8::Initialize();

    create_params_.array_buffer_allocator =
        v8::ArrayBuffer::Allocator::NewDefaultAllocator();
  }

  const v8::Isolate::CreateParams &get_create_params() const {
    return create_params_;
  }
  const v8::CompiledWasmModule &get_compiled_wasm() const {
    return wasm_compiled_.value();
  }
  const v8::CompiledWasmModule &compile(std::span<uint8_t> bin) {
    v8::Isolate *isolate = v8::Isolate::New(create_params_);
    {
      v8::Isolate::Scope isolate_scope(isolate);
      v8::HandleScope handle_scope(isolate);
      v8::Local<v8::Context> context = v8::Context::New(isolate);
      v8::Context::Scope context_scope(context);

      v8::Local<v8::WasmModuleObject> module =
          v8::WasmModuleObject::Compile(
              isolate, v8::MemorySpan<const uint8_t>(bin.data(), bin.size()))
              .ToLocalChecked();

      wasm_compiled_.emplace(module->GetCompiledModule());
    }
    isolate->Dispose();

    return wasm_compiled_.value();
  }

  ~V8Env() {
    wasm_compiled_.reset();
    v8::V8::Dispose();
    v8::V8::DisposePlatform();
    delete create_params_.array_buffer_allocator;
  }
};
