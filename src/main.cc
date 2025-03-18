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

int main(int argc, char *argv[]) {
  OptionParser parser(argv[0], "v8 benchmarking");
  bool bounds_checks = false;
  size_t number_of_threads = 1;
  size_t requests_per_second = 1000;
  bool use_w2c = false;
  string wasm_path;

  parser.AddArgument("wasm-binary", OptionParser::ArgumentCount::One,
                     [&](const char *arg) { wasm_path = string(arg); });
  parser.AddOption('b', "bounds-check", "enforce wasm bounds-check",
                   [&] { bounds_checks = true; });
  parser.AddOption('j', "parallel", "threads", "number of worker threads",
                   [&](const char *arg) { number_of_threads = stoi(arg); });
  parser.AddOption('r', "request-rate", "rate", "number of requests per second",
                   [&](const char *arg) { requests_per_second = stoi(arg); });
  parser.AddOption("wasm2c", "benchmark wasm2c", [&]() { use_w2c = true; });

  parser.Parse(argc, argv);

  int result;

  if (use_w2c) {
    if (bounds_checks) {
      unique_ptr<W2CDirectRuntime<AddRequest, W2Caddboundscheck>> runner{
          make_unique<W2CDirectRuntime<AddRequest, W2Caddboundscheck>>(
              number_of_threads)};
      runner->start();
      sleep(10);
      result = runner->report();
      runner.reset();
    } else {
      unique_ptr<W2CDirectRuntime<AddRequest, W2Caddmmap>> runner{
          make_unique<W2CDirectRuntime<AddRequest, W2Caddmmap>>(
              number_of_threads)};
      runner->start();
      sleep(10);
      result = runner->report();
      runner.reset();
    }
  } else {
    ReadOnlyFile in(wasm_path);
    unique_ptr<V8DirectRuntime<AddRequest>> runner{
        make_unique<V8DirectRuntime<AddRequest>>(
            argv[0], bounds_checks,
            span<uint8_t>(reinterpret_cast<uint8_t *>(in.addr()), in.length()),
            number_of_threads)};
    runner->start();
    sleep(10);
    result = runner->report();
    runner.reset();
  }

  cout << "Total request processed: " << result << endl;

  return 0;
}
