#ifndef VM_FFI_H
#define VM_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// 定义FFIGCRef结构体
typedef struct {
    void* data;   // 指向对象数据的指针
    void* vtable; // 指向类型虚表的指针
} FFIGCRef;

// 定义空引用
static inline FFIGCRef FFIGCRef_null() {
    FFIGCRef ref;
    ref.data = NULL;
    ref.vtable = NULL;
    return ref;
}

// 定义Rust查找函数的类型
typedef void* (*RustLookupFn)(const char* name);

// CLambda 入口和销毁函数
void* clambda_entry(void* lookup_fn);
void clambda_destroy(void);

// CLambda函数体类型
typedef FFIGCRef (*CLambdaBodyFn)(FFIGCRef gc_ref, void* gc_system);

// 存储Rust查找函数
extern RustLookupFn rust_lookup;

//==============================================================
// VM 对象创建函数
//==============================================================

// 创建整数对象
FFIGCRef new_vm_int64(int64_t value, void* gc_system);

// 创建浮点数对象
FFIGCRef new_vm_float64(double value, void* gc_system);

// 创建字符串对象
FFIGCRef new_vm_string(const char* str, void* gc_system);

// 创建布尔值对象
FFIGCRef new_vm_boolean(int value, void* gc_system);

// 创建空对象
FFIGCRef new_vm_null(void* gc_system);

// 创建字节数组对象
FFIGCRef new_vm_bytes(const uint8_t* data, int len, void* gc_system);

// 创建元组对象
FFIGCRef new_vm_tuple(void* gc_system);

// 创建键值对对象
FFIGCRef new_vm_keyval(FFIGCRef key, FFIGCRef value, void* gc_system);

// 创建命名对象
FFIGCRef new_vm_named(FFIGCRef key, FFIGCRef value, void* gc_system);

// 创建包装对象
FFIGCRef new_vm_wrapper(FFIGCRef value, void* gc_system);

//==============================================================
// 类型检查函数
//==============================================================

// 检查是否为整数对象
int is_vm_int(FFIGCRef obj);

// 检查是否为浮点数对象
int is_vm_float(FFIGCRef obj);

// 检查是否为字符串对象
int is_vm_string(FFIGCRef obj);

// 检查是否为布尔值对象
int is_vm_boolean(FFIGCRef obj);

// 检查是否为空对象
int is_vm_null(FFIGCRef obj);

// 检查是否为字节数组对象
int is_vm_bytes(FFIGCRef obj);

// 检查是否为元组对象
int is_vm_tuple(FFIGCRef obj);

// 检查是否为键值对对象
int is_vm_keyval(FFIGCRef obj);

// 检查是否为命名对象
int is_vm_named(FFIGCRef obj);

// 检查是否为包装对象
int is_vm_wrapper(FFIGCRef obj);

//==============================================================
// 值获取函数
//==============================================================

// 获取整数值
int64_t get_vm_int_value(FFIGCRef obj);

// 获取浮点数值
double get_vm_float_value(FFIGCRef obj);

// 获取字符串值
char* get_vm_string_value(FFIGCRef obj);

// 获取布尔值
int get_vm_boolean_value(FFIGCRef obj);

//==============================================================
// 集合操作函数
//==============================================================

// 向元组添加元素
int vm_tuple_append(FFIGCRef tuple, FFIGCRef value);

// 获取元组中的元素
FFIGCRef vm_tuple_get(FFIGCRef tuple, int index, void* gc_system);

//==============================================================
// 对象操作函数
//==============================================================

// 获取对象的值
FFIGCRef get_vm_value(FFIGCRef obj);

// 获取对象的键
FFIGCRef get_vm_key(FFIGCRef obj);

// 设置对象的值
int set_vm_value(FFIGCRef target, FFIGCRef value);

// 获取对象的长度
int get_len(FFIGCRef obj);

// 引用计数操作
int clone_ref(FFIGCRef obj);
int drop_ref(FFIGCRef obj);

#ifdef __cplusplus
}
#endif

#endif // VM_FFI_H