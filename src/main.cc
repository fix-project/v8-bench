#include <chrono>
#include <cstdio>
#include <iostream>
#include <memory>
#include <stdlib.h>

#include "mmap.hh"
#include "option-parser.hh"
#include "v8/v8runner.hh"
#include "wasm2c/w2cinstance.hh"
#include "wasm2c/w2crunner.hh"

using args_type = v8::Local<v8::Value>[];
using namespace std;

class AddRequest {
private:
  constexpr static array<int, 2> args_ = {77, 88};

public:
  const char *func() { return "add"; }

  span<int> args() {
    return span<int>(const_cast<int *>(args_.data()), args_.size());
  }
};

enum RunnerMode {
  W2CNoMem,
  W2CSW,
  W2CHW,
  V8,
  V8NewIsolate,
  V8NewContext,
};

int main(int argc, char *argv[]) {
  OptionParser parser(argv[0], "v8 benchmarking");
  size_t number_of_threads = 1;

  RunnerMode runner_mode = RunnerMode::W2CNoMem;
  optional<string> wasm_path;

  parser.AddOption('r', "runner-mode",
                   "w2c-no-mem|w2c-sw|w2c-hw|v8|v8-new-isolate|v8-new-context",
                   "Mode of runners (default: w2c-no-mem)",
                   [&](const char *arg) {
                     if (strcmp(arg, "w2c-no-mem") == 0) {
                       runner_mode = RunnerMode::W2CNoMem;
                     } else if (strcmp(arg, "w2c-sw") == 0) {
                       runner_mode = RunnerMode::W2CSW;
                     } else if (strcmp(arg, "w2c-hw") == 0) {
                       runner_mode = RunnerMode::W2CHW;
                     } else if (strcmp(arg, "v8") == 0) {
                       runner_mode = RunnerMode::V8;
                     } else if (strcmp(arg, "v8-new-isolate") == 0) {
                       runner_mode = RunnerMode::V8NewIsolate;
                     } else if (strcmp(arg, "v8-new-context") == 0) {
                       runner_mode = RunnerMode::V8NewContext;
                     } else {
                       cerr << "Bad argument\n";
                       abort();
                     }
                   });

  parser.AddOption(
      'b', "wasm-binary", "PATH",
      "path to wasm binary, must be specificed for v8/v8-new-isolate",
      [&](const char *arg) { wasm_path = string(arg); });

  parser.AddOption('j', "parallel", "threads", "number of worker threads",
                   [&](const char *arg) { number_of_threads = stoi(arg); });

  parser.Parse(argc, argv);

  if ((runner_mode == RunnerMode::V8NewIsolate ||
       runner_mode == RunnerMode::V8) &&
      (!wasm_path.has_value())) {
    cerr << "Wasm binary not specified\n";
    abort();
  }

  unique_ptr<Runtime> runner;
  switch (runner_mode) {
  case W2CNoMem:
    runner =
        make_unique<W2CDirectRuntime<AddRequest, W2Cadd>>(number_of_threads);
    break;

  case W2CSW:
    runner = make_unique<W2CDirectRuntime<AddRequest, W2Caddboundscheck>>(
        number_of_threads);
    break;

  case W2CHW:
    runner = make_unique<W2CDirectRuntime<AddRequest, W2Caddmmap>>(
        number_of_threads);
    break;

  case V8: {
    ReadOnlyFile in(wasm_path.value());
    runner = make_unique<V8DirectRuntime<AddRequest>>(
        argv[0], false, false,
        span<uint8_t>(reinterpret_cast<uint8_t *>(in.addr()), in.length()),
        number_of_threads);
    break;
  }

  case V8NewIsolate: {
    ReadOnlyFile in(wasm_path.value());
    runner = make_unique<V8DirectRuntime<AddRequest>>(
        argv[0], true, false,
        span<uint8_t>(reinterpret_cast<uint8_t *>(in.addr()), in.length()),
        number_of_threads);
    break;
  }

  case V8NewContext: {
    ReadOnlyFile in(wasm_path.value());
    runner = make_unique<V8DirectRuntime<AddRequest>>(
        argv[0], false, true,
        span<uint8_t>(reinterpret_cast<uint8_t *>(in.addr()), in.length()),
        number_of_threads);
    break;
  }
  }

  auto now = chrono::steady_clock::now();
  runner->start();
  sleep(10);
  auto result = runner->report();
  auto end = chrono::steady_clock::now();
  auto elapsed = end - now;
  auto time = elapsed / result;
  auto iter_per_second =
      (double)result / duration_cast<chrono::seconds>(elapsed).count();

  cout << number_of_threads << " threads: " << time << " per iteration ("
       << iter_per_second << " iters/second) - ran for " << elapsed << endl;

  return 0;
}
