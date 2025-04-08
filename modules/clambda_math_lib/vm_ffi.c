#include "vm_ffi.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// 存储Rust查找函数
RustLookupFn rust_lookup = NULL;

// CLambda 入口函数 - 初始化库
void* clambda_entry(void* lookup_fn) {
    rust_lookup = (RustLookupFn)lookup_fn;
    return NULL;
}

// CLambda 销毁函数 - 清理资源
void clambda_destroy(void) {
    rust_lookup = NULL;
}

// 辅助函数 - 调用Rust函数
static void* call_rust_function(const char* name, ...) {
    if (!rust_lookup) {
        printf("Error: rust_lookup not initialized\n");
        return NULL;
    }
    
    // 使用rust_lookup查找函数
    return rust_lookup(name);
}

// VM 对象创建函数的包装
FFIGCRef new_vm_int64(int64_t value, void* gc_system) {
    FFIGCRef (*func)(int64_t, void*) = (FFIGCRef (*)(int64_t, void*))call_rust_function("new_vm_int64");
    if (func) {
        return func(value, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_float64(double value, void* gc_system) {
    FFIGCRef (*func)(double, void*) = (FFIGCRef (*)(double, void*))call_rust_function("new_vm_float64");
    if (func) {
        return func(value, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_string(const char* str, void* gc_system) {
    FFIGCRef (*func)(const char*, void*) = (FFIGCRef (*)(const char*, void*))call_rust_function("new_vm_string");
    if (func) {
        return func(str, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_boolean(int value, void* gc_system) {
    FFIGCRef (*func)(int, void*) = (FFIGCRef (*)(int, void*))call_rust_function("new_vm_boolean");
    if (func) {
        return func(value, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_null(void* gc_system) {
    FFIGCRef (*func)(void*) = (FFIGCRef (*)(void*))call_rust_function("new_vm_null");
    if (func) {
        return func(gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_bytes(const uint8_t* data, int len, void* gc_system) {
    FFIGCRef (*func)(const uint8_t*, int, void*) = 
        (FFIGCRef (*)(const uint8_t*, int, void*))call_rust_function("new_vm_bytes");
    if (func) {
        return func(data, len, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_tuple(void* gc_system) {
    FFIGCRef (*func)(void*) = (FFIGCRef (*)(void*))call_rust_function("new_vm_tuple");
    if (func) {
        return func(gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_keyval(FFIGCRef key, FFIGCRef value, void* gc_system) {
    FFIGCRef (*func)(FFIGCRef, FFIGCRef, void*) = 
        (FFIGCRef (*)(FFIGCRef, FFIGCRef, void*))call_rust_function("new_vm_keyval");
    if (func) {
        return func(key, value, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_named(FFIGCRef key, FFIGCRef value, void* gc_system) {
    FFIGCRef (*func)(FFIGCRef, FFIGCRef, void*) = 
        (FFIGCRef (*)(FFIGCRef, FFIGCRef, void*))call_rust_function("new_vm_named");
    if (func) {
        return func(key, value, gc_system);
    }
    return FFIGCRef_null();
}

FFIGCRef new_vm_wrapper(FFIGCRef value, void* gc_system) {
    FFIGCRef (*func)(FFIGCRef, void*) = 
        (FFIGCRef (*)(FFIGCRef, void*))call_rust_function("new_vm_wrapper");
    if (func) {
        return func(value, gc_system);
    }
    return FFIGCRef_null();
}

// 类型检查函数的包装
int is_vm_int(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_int");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_float(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_float");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_string(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_string");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_boolean(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_boolean");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_null(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_null");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_bytes(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_bytes");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_tuple(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_tuple");
    if (func) {

        return func(obj);
    }
    return 0;
}

int is_vm_keyval(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_keyval");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_named(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_named");
    if (func) {
        return func(obj);
    }
    return 0;
}

int is_vm_wrapper(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("is_vm_wrapper");
    if (func) {
        return func(obj);
    }
    return 0;
}

// 值获取函数的包装
int64_t get_vm_int_value(FFIGCRef obj) {
    int64_t (*func)(FFIGCRef) = (int64_t (*)(FFIGCRef))call_rust_function("get_vm_int_value");
    if (func) {
        return func(obj);
    }
    return 0;
}

double get_vm_float_value(FFIGCRef obj) {
    double (*func)(FFIGCRef) = (double (*)(FFIGCRef))call_rust_function("get_vm_float_value");
    if (func) {
        return func(obj);
    }
    return 0.0;
}

char* get_vm_string_value(FFIGCRef obj) {
    char* (*func)(FFIGCRef) = (char* (*)(FFIGCRef))call_rust_function("get_vm_string_value");
    if (func) {
        return func(obj);
    }
    return NULL;
}

int get_vm_boolean_value(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("get_vm_boolean_value");
    if (func) {
        return func(obj);
    }
    return 0;
}

// 集合操作函数包装
int vm_tuple_append(FFIGCRef tuple, FFIGCRef value) {
    int (*func)(FFIGCRef, FFIGCRef) = (int (*)(FFIGCRef, FFIGCRef))call_rust_function("vm_tuple_append");
    if (func) {
        return func(tuple, value);
    }
    return 0;
}

FFIGCRef vm_tuple_get(FFIGCRef tuple, int index, void* gc_system) {
    FFIGCRef (*func)(FFIGCRef, int, void*) = 
        (FFIGCRef (*)(FFIGCRef, int, void*))call_rust_function("vm_tuple_get");
    if (func) {
        return func(tuple, index, gc_system);
    }
    return FFIGCRef_null();
}

// 对象操作函数包装
FFIGCRef get_vm_value(FFIGCRef obj) {
    FFIGCRef (*func)(FFIGCRef) = (FFIGCRef (*)(FFIGCRef))call_rust_function("get_vm_value");
    if (func) {
        return func(obj);
    }
    return FFIGCRef_null();
}

FFIGCRef get_vm_key(FFIGCRef obj) {
    FFIGCRef (*func)(FFIGCRef) = (FFIGCRef (*)(FFIGCRef))call_rust_function("get_vm_key");
    if (func) {
        return func(obj);
    }
    return FFIGCRef_null();
}

int set_vm_value(FFIGCRef target, FFIGCRef value) {
    int (*func)(FFIGCRef, FFIGCRef) = (int (*)(FFIGCRef, FFIGCRef))call_rust_function("set_vm_value");
    if (func) {
        return func(target, value);
    }
    return 0;
}

int get_len(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("get_len");
    if (func) {
        return func(obj);
    }
    return 0;
}

int clone_ref(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("clone_ref");
    if (func) {
        return func(obj);
    }
    return 0;
}

int drop_ref(FFIGCRef obj) {
    int (*func)(FFIGCRef) = (int (*)(FFIGCRef))call_rust_function("drop_ref");
    if (func) {
        return func(obj);
    }
    return 0;
}

// 处理数据并打印信息的辅助函数
static void print_value_info(FFIGCRef obj) {
    if (is_vm_int(obj)) {
        printf("  整数值: %lld\n", (long long)get_vm_int_value(obj));
    } 
    else if (is_vm_float(obj)) {
        printf("  浮点值: %f\n", get_vm_float_value(obj));
    }
    else if (is_vm_string(obj)) {
        char* str = get_vm_string_value(obj);
        if (str) {
            printf("  字符串值: '%s'\n", str);
            free(str); // 记得释放字符串
        } else {
            printf("  字符串值: <无法获取>\n");
        }
    }
    else if (is_vm_boolean(obj)) {
        printf("  布尔值: %s\n", get_vm_boolean_value(obj) ? "true" : "false");
    }
    else if (is_vm_null(obj)) {
        printf("  空值\n");
    }
    else if (is_vm_tuple(obj)) {
        printf("  元组类型\n");
    }
    else if (is_vm_keyval(obj)) {
        printf("  键值对类型\n");
    }
    else if (is_vm_named(obj)) {
        printf("  命名对象类型\n");
    }
    else if (is_vm_wrapper(obj)) {
        printf("  包装对象类型\n");
    }
    else if (is_vm_bytes(obj)) {
        printf("  字节数组类型\n");
    }
    else {
        printf("  未知类型\n");
        printf("  对象地址: %p\n", obj.data);
        printf("  对象Vtable: %p\n", obj.vtable);
    }
}


