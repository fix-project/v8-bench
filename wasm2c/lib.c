#include <stdio.h>
#include <stdlib.h>

#include "module.h"

void *allocate_module() {
  w2c_module *module = malloc(sizeof(w2c_module));
  wasm2c_module_instantiate(module);
  return module;
}

void free_module(void *module) {
  wasm2c_module_free(module);
}

