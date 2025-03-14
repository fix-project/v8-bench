#include <memory>
#include <stdlib.h>

#include "mmap.hh"
#include "option-parser.hh"
#include "v8/v8runner.hh"

using args_type = v8::Local<v8::Value>[];
using namespace std;

class AddRequest {
  private:
    constexpr static array<int, 2> args_ = { 77, 88 };
  public:
    const char* func() {
      return "add";
    }

    span<const int> args() {
      return span<const int>( args_.data(), args_.size() );
    }
};

int main(int argc, char *argv[]) {
  OptionParser parser(argv[0], "v8 benchmarking");
  bool bounds_checks = false;
  size_t number_of_threads = 1;
  size_t requests_per_second = 1000;
  string wasm_path;

  parser.AddArgument("wasm-binary", OptionParser::ArgumentCount::One,
                     [&](const char *arg) { wasm_path = string(arg); });
  parser.AddOption('b', "bounds-check", "enforce wasm bounds-check",
                   [&] { bounds_checks = true; });
  parser.AddOption('j', "parallel", "threads", "number of worker threads",
                   [&](const char *arg) { number_of_threads = stoi(arg); });
  parser.AddOption('r', "request-rate", "rate", "number of requests per second",
                   [&](const char *arg) { requests_per_second = stoi(arg); });

  parser.Parse(argc, argv);

  ReadOnlyFile in(wasm_path);

  unique_ptr<V8Runtime<AddRequest>> runner{make_unique<V8Runtime<AddRequest>>(
      argv[0], bounds_checks,
      span<uint8_t>(reinterpret_cast<uint8_t *>(in.addr()), in.length()),
      number_of_threads, requests_per_second)};
  runner->start();
  sleep( 10 );
  runner.reset();

  return 0;
}
