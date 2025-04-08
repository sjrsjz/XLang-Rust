
#include "vm_ffi.h"
#include "math_lib.h"
#include <stdlib.h>
#include <stdio.h>
#include <math.h>

// 辅助函数 - 从元组获取单个浮点数
static double get_float_from_tuple(FFIGCRef tuple, int index, void* gc_system) {
    FFIGCRef value = vm_tuple_get(tuple, index, gc_system);
    
    if (!value.data) {
        printf("Error: Failed to get tuple element at index %d\n", index);
        return 0.0;
    }
    
    if (is_vm_float(value)) {
        return get_vm_float_value(value);
    } else if (is_vm_int(value)) {
        return (double)get_vm_int_value(value);
    } else {
        printf("Error: Expected numeric value at index %d\n", index);
        return 0.0;
    }
}

// 辅助函数 - 检查元组参数数量
static int check_args_count(FFIGCRef tuple, int expected) {
    if (!is_vm_tuple(tuple)) {
        printf("Error: Expected a tuple\n");
        return 0;
    }
    
    int size = get_len(tuple);
    if (size != expected) {
        printf("Error: Expected %d arguments, got %d\n", expected, size);
        return 0;
    }
    
    return 1;
}

// 三角函数

// sin 函数
FFIGCRef clambda_sin(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = sin(x);
    
    return new_vm_float64(result, gc_system);
}

// cos 函数
FFIGCRef clambda_cos(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = cos(x);
    
    return new_vm_float64(result, gc_system);
}

// tan 函数
FFIGCRef clambda_tan(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = tan(x);
    
    return new_vm_float64(result, gc_system);
}

// 乘方和开方

// pow 函数
FFIGCRef clambda_pow(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 2)) {
        return FFIGCRef_null();
    }
    
    double base = get_float_from_tuple(gc_ref, 0, gc_system);
    double exponent = get_float_from_tuple(gc_ref, 1, gc_system);
    double result = pow(base, exponent);
    
    return new_vm_float64(result, gc_system);
}

// sqrt 函数
FFIGCRef clambda_sqrt(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    if (x < 0) {
        printf("Warning: Taking square root of negative number\n");
    }
    double result = sqrt(x);
    
    return new_vm_float64(result, gc_system);
}

// 舍入函数

// round 函数
FFIGCRef clambda_round(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = round(x);
    
    return new_vm_float64(result, gc_system);
}

// floor 函数
FFIGCRef clambda_floor(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = floor(x);
    
    return new_vm_float64(result, gc_system);
}

// ceil 函数
FFIGCRef clambda_ceil(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = ceil(x);
    
    return new_vm_float64(result, gc_system);
}

// 对数函数

// log 函数 (自然对数)
FFIGCRef clambda_log(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    if (x <= 0) {
        printf("Error: Cannot take logarithm of non-positive number\n");
        return FFIGCRef_null();
    }
    double result = log(x);
    
    return new_vm_float64(result, gc_system);
}

// log10 函数
FFIGCRef clambda_log10(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    if (x <= 0) {
        printf("Error: Cannot take logarithm of non-positive number\n");
        return FFIGCRef_null();
    }
    double result = log10(x);
    
    return new_vm_float64(result, gc_system);
}

// 指数函数
FFIGCRef clambda_exp(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = exp(x);
    
    return new_vm_float64(result, gc_system);
}

// 最值函数

// max 函数
FFIGCRef clambda_max(FFIGCRef gc_ref, void* gc_system) {
    if (!is_vm_tuple(gc_ref)) {
        printf("Error: Expected a tuple\n");
        return FFIGCRef_null();
    }
    
    int size = get_len(gc_ref);
    if (size < 1) {
        printf("Error: Expected at least one argument\n");
        return FFIGCRef_null();
    }
    
    double max_val = get_float_from_tuple(gc_ref, 0, gc_system);
    
    for (int i = 1; i < size; i++) {
        double val = get_float_from_tuple(gc_ref, i, gc_system);
        if (val > max_val) {
            max_val = val;
        }
    }
    
    return new_vm_float64(max_val, gc_system);
}

// min 函数
FFIGCRef clambda_min(FFIGCRef gc_ref, void* gc_system) {
    if (!is_vm_tuple(gc_ref)) {
        printf("Error: Expected a tuple\n");
        return FFIGCRef_null();
    }
    
    int size = get_len(gc_ref);
    if (size < 1) {
        printf("Error: Expected at least one argument\n");
        return FFIGCRef_null();
    }
    
    double min_val = get_float_from_tuple(gc_ref, 0, gc_system);
    
    for (int i = 1; i < size; i++) {
        double val = get_float_from_tuple(gc_ref, i, gc_system);
        if (val < min_val) {
            min_val = val;
        }
    }
    
    return new_vm_float64(min_val, gc_system);
}

// abs 函数
FFIGCRef clambda_abs(FFIGCRef gc_ref, void* gc_system) {
    if (!check_args_count(gc_ref, 1)) {
        return FFIGCRef_null();
    }
    
    double x = get_float_from_tuple(gc_ref, 0, gc_system);
    double result = fabs(x);
    
    return new_vm_float64(result, gc_system);
}

// PI 和 E 常量
FFIGCRef clambda_pi(FFIGCRef gc_ref, void* gc_system) {
    return new_vm_float64(M_PI, gc_system);
}

FFIGCRef clambda_e(FFIGCRef gc_ref, void* gc_system) {
    return new_vm_float64(M_E, gc_system);
}