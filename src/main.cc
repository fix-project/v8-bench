#include <memory>
#include <stdlib.h>

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

int main(int, char *argv[]) {
  std::vector<uint8_t> wasmbin{
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01,
      0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07,
      0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, 0x0a, 0x09, 0x01,
      0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b};

  unique_ptr<V8Runtime<AddRequest>> runner { make_unique<V8Runtime<AddRequest>>( argv, span<uint8_t>( wasmbin ), 2,100 ) };
  runner->start();
  sleep( 10 );
  runner.reset();

  return 0;
}
