#include <stdio.h>
#include <stdlib.h>
#include <thread>

#include "v8/v8env.hh"
#include "v8/v8instance.hh"

using args_type = v8::Local<v8::Value>[];
using namespace std;

int main(int, char *argv[]) {
  V8Env env(argv);
  std::vector<uint8_t> wasmbin{
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01,
      0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07,
      0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, 0x0a, 0x09, 0x01,
      0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b};
  env.compile(wasmbin);

  auto fn = [&]() {
    V8Instance inst(env);
    auto module_instance = inst.instantiate();
    auto res = inst.invoke(std::move(module_instance), "add", {77, 88});
    auto number = inst.to_uint32(std::move(res));
    printf("77 + 88 = %u\n", number);

    auto new_res = inst.invoke(inst.instantiate(), "add", {44, 55});
    auto new_number = inst.to_uint32(std::move(new_res));
    printf("44 + 55= %u\n", new_number);
  };

  auto t1 = thread(fn);
  auto t2 = thread(fn);
  t1.join();
  t2.join();

  return 0;
}
