#ifndef MATH_LIB_H
#define MATH_LIB_H

#include "vm_ffi.h"

#ifdef __cplusplus
extern "C" {
#endif

#define M_PI 3.14159265358979323846
#define M_E  2.71828182845904523536

// 三角函数
FFIGCRef clambda_sin(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_cos(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_tan(FFIGCRef gc_ref, void* gc_system);

// 乘方和开方
FFIGCRef clambda_pow(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_sqrt(FFIGCRef gc_ref, void* gc_system);

// 舍入函数
FFIGCRef clambda_round(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_floor(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_ceil(FFIGCRef gc_ref, void* gc_system);

// 对数函数
FFIGCRef clambda_log(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_log10(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_exp(FFIGCRef gc_ref, void* gc_system);

// 最值函数
FFIGCRef clambda_max(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_min(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_abs(FFIGCRef gc_ref, void* gc_system);

// 常量
FFIGCRef clambda_pi(FFIGCRef gc_ref, void* gc_system);
FFIGCRef clambda_e(FFIGCRef gc_ref, void* gc_system);

#ifdef __cplusplus
}
#endif

#endif // MATH_LIB_H