#pragma once

#include "v8env.hh"
#include <v8-context.h>
#include <v8-persistent-handle.h>
#include <v8-primitive.h>
#include <v8-wasm.h>

struct IsolateWrapper {
  v8::Isolate *isolate_;
  IsolateWrapper(v8::Isolate *isolate);
  ~IsolateWrapper();
};

class V8Instance {
private:
  V8Env &env_;

  IsolateWrapper isolate_;
  v8::Isolate::Scope isolate_scope_;
  v8::HandleScope handle_scope_;
  v8::Local<v8::Context> context_;
  v8::Local<v8::Value> module_;

public:
  V8Instance(V8Env &env);
  V8Instance(V8Env &env, v8::CompiledWasmModule module);
  // Instanciate a wasm module instance
  v8::UniquePersistent<v8::Object> instantiate();
  // Comsume a wasm module instance and invoke the specified function
  v8::UniquePersistent<v8::Value>
  invoke(v8::UniquePersistent<v8::Object> &&module_instance, const char *func,
         std::span<const int> args);

  v8::UniquePersistent<v8::Value>
  instantiate_and_invoke(const char *func, std::span<const int> args);
  v8::UniquePersistent<v8::Value>
  instantiate_and_invoke_new_context(const char *func,
                                     std::span<const int> args);

  uint32_t to_uint32(v8::UniquePersistent<v8::Value> &&handle);
};
