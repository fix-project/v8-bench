/* Automatically generated by wasm2c */
#ifndef ADD_H_GENERATED_
#define ADD_H_GENERATED_

#include <stdint.h>

#include "wasm-rt.h"

#if defined(WASM_RT_ENABLE_SIMD)
#include "simde/wasm/simd128.h"
#endif

/* TODO(binji): only use stdint.h types in header */
#ifndef WASM_RT_CORE_TYPES_DEFINED
#define WASM_RT_CORE_TYPES_DEFINED
typedef uint8_t u8;
typedef int8_t s8;
typedef uint16_t u16;
typedef int16_t s16;
typedef uint32_t u32;
typedef int32_t s32;
typedef uint64_t u64;
typedef int64_t s64;
typedef float f32;
typedef double f64;

#if defined(WASM_RT_ENABLE_SIMD)
typedef simde_v128_t v128;
#endif

#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct w2c_add {
  char dummy_member;
} w2c_add;

void wasm2c_add_instantiate(w2c_add *);
void wasm2c_add_free(w2c_add *);
wasm_rt_func_type_t wasm2c_add_get_func_type(uint32_t param_count,
                                             uint32_t result_count, ...);

/* export: 'add' */
u32 w2c_add_add(w2c_add *, u32, u32);

#ifdef __cplusplus
}
#endif

#endif /* ADD_H_GENERATED_ */
