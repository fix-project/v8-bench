#pragma once

#include "add.h"
#include "addhw.h"
#include "addsw.h"

#include <span>

template <typename T, void instantiate(T *), void free(T *),
          u32 call(T *, u32, u32)>
class W2CInstance {
private:
  T *instance_{};

public:
  W2CInstance() {
    instance_ = new T();
    instantiate(instance_);
  }

  ~W2CInstance() {
    free(instance_);
    delete instance_;
  }

  int invoke(std::span<int> args) { return call(instance_, args[0], args[1]); }

  W2CInstance(const W2CInstance &) = delete;
  W2CInstance &operator=(const W2CInstance &) = delete;
};

using W2Cadd =
    W2CInstance<w2c_add, wasm2c_add_instantiate, wasm2c_add_free, w2c_add_add>;
using W2Caddboundscheck = W2CInstance<w2c_addsw, wasm2c_addsw_instantiate,
                                      wasm2c_addsw_free, w2c_addsw_add>;
using W2Caddmmap = W2CInstance<w2c_addhw, wasm2c_addhw_instantiate,
                               wasm2c_addhw_free, w2c_addhw_add>;
