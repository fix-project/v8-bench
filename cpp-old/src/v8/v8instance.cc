#include "v8instance.hh"
#include <v8-context.h>
#include <v8-persistent-handle.h>
#include <v8-wasm.h>

using namespace v8;
using namespace std;

IsolateWrapper::IsolateWrapper(Isolate *isolate) : isolate_(isolate) {}

IsolateWrapper::~IsolateWrapper() { isolate_->Dispose(); }

V8Instance::V8Instance(V8Env &env)
    : env_(env), isolate_(Isolate::New(env_.get_create_params())),
      isolate_scope_(isolate_.isolate_), handle_scope_(isolate_.isolate_),
      context_(Context::New(isolate_.isolate_)), module_() {
  context_->Enter();
  module_ = WasmModuleObject::FromCompiledModule(isolate_.isolate_,
                                                 env_.get_compiled_wasm())
                .ToLocalChecked();
  context_->Exit();
}

V8Instance::V8Instance(V8Env &env, v8::CompiledWasmModule module)
    : env_(env), isolate_(Isolate::New(env_.get_create_params())),
      isolate_scope_(isolate_.isolate_), handle_scope_(isolate_.isolate_),
      context_(Context::New(isolate_.isolate_)), module_() {
  context_->Enter();
  module_ = WasmModuleObject::FromCompiledModule(isolate_.isolate_, module)
                .ToLocalChecked();
  context_->Exit();
}

UniquePersistent<Object> V8Instance::instantiate() {
  Isolate *isolate = isolate_.isolate_;
  HandleScope scope(isolate);
  context_->Enter();

  Local<Object> module_instance =
      context_->Global()
          ->Get(context_,
                String::NewFromUtf8(isolate, "WebAssembly").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context_,
                String::NewFromUtf8(isolate, "Instance").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->CallAsConstructor(context_, 1, &module_)
          .ToLocalChecked()
          .As<Object>();

  context_->Exit();

  return UniquePersistent<Object>(isolate, module_instance);
}

UniquePersistent<Value>
V8Instance::invoke(UniquePersistent<Object> &&module_instance, const char *func,
                   span<const int> args) {
  Isolate *isolate = isolate_.isolate_;
  HandleScope scope(isolate);
  context_->Enter();

  vector<Local<Value>> v8_args;
  for (const auto &arg : args) {
    v8_args.push_back(Int32::New(isolate, arg));
  }

  auto v8_func =
      module_instance.Get(isolate)
          ->Get(context_,
                String::NewFromUtf8(isolate, "exports").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context_, String::NewFromUtf8(isolate, func).ToLocalChecked())
          .ToLocalChecked()
          .As<Function>();

  auto res =
      v8_func->Call(context_, context_->Global(), v8_args.size(), &v8_args[0])
          .ToLocalChecked();

  context_->Exit();

  return UniquePersistent<Value>(isolate, res);
}

UniquePersistent<Value>
V8Instance::instantiate_and_invoke(const char *func,
                                   std::span<const int> args) {
  Isolate *isolate = isolate_.isolate_;
  HandleScope scope(isolate);
  context_->Enter();

  Local<Object> module_instance =
      context_->Global()
          ->Get(context_,
                String::NewFromUtf8(isolate, "WebAssembly").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context_,
                String::NewFromUtf8(isolate, "Instance").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->CallAsConstructor(context_, 1, &module_)
          .ToLocalChecked()
          .As<Object>();

  vector<Local<Value>> v8_args;
  for (const auto &arg : args) {
    v8_args.push_back(Int32::New(isolate, arg));
  }

  auto v8_func =
      module_instance
          ->Get(context_,
                String::NewFromUtf8(isolate, "exports").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context_, String::NewFromUtf8(isolate, func).ToLocalChecked())
          .ToLocalChecked()
          .As<Function>();

  auto res =
      v8_func->Call(context_, context_->Global(), v8_args.size(), &v8_args[0])
          .ToLocalChecked();

  context_->Exit();

  return UniquePersistent<Value>(isolate, res);
}

UniquePersistent<Value>
V8Instance::instantiate_and_invoke_new_context(const char *func,
                                               std::span<const int> args) {
  Isolate *isolate = isolate_.isolate_;
  HandleScope scope(isolate);

  Local<Context> context = Context::New(isolate);
  Context::Scope context_scope(context);

  Local<Object> module_instance =
      context->Global()
          ->Get(context,
                String::NewFromUtf8(isolate, "WebAssembly").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context,
                String::NewFromUtf8(isolate, "Instance").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->CallAsConstructor(context, 1, &module_)
          .ToLocalChecked()
          .As<Object>();

  vector<Local<Value>> v8_args;
  for (const auto &arg : args) {
    v8_args.push_back(Int32::New(isolate, arg));
  }

  auto v8_func =
      module_instance
          ->Get(context,
                String::NewFromUtf8(isolate, "exports").ToLocalChecked())
          .ToLocalChecked()
          .As<Object>()
          ->Get(context, String::NewFromUtf8(isolate, func).ToLocalChecked())
          .ToLocalChecked()
          .As<Function>();

  auto res =
      v8_func->Call(context, context->Global(), v8_args.size(), &v8_args[0])
          .ToLocalChecked();

  return UniquePersistent<Value>(isolate, res);
}

uint32_t V8Instance::to_uint32(UniquePersistent<Value> &&handle) {
  Isolate *isolate = isolate_.isolate_;
  HandleScope scope(isolate);
  context_->Enter();
  auto res = handle.Get(isolate)->Uint32Value(context_).ToChecked();
  context_->Exit();
  return res;
}
