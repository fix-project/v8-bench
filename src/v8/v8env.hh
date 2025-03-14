#pragma once

#include "include/v8.h"
#include "v8-wasm.h"
#include <span>

class V8Env {
private:
  v8::Isolate::CreateParams create_params_;
  std::optional<v8::CompiledWasmModule> wasm_compiled_;
  std::unique_ptr<v8::Platform> platform_;

public:
  V8Env(char *argv0, bool bounds_checks = false);
  const v8::Isolate::CreateParams &get_create_params() const;
  const v8::CompiledWasmModule &get_compiled_wasm() const;
  const v8::CompiledWasmModule &compile(std::span<uint8_t> bin);
  ~V8Env();
};
