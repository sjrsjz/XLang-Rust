// // 在现有导入列表中添加
// use libloading::{Library, Symbol};
// use std::ffi::{c_void, CString, CStr};
// use std::os::raw::{c_char, c_int};

// // 定义FFI调用的函数签名类型
// type InitFn = unsafe extern "C" fn() -> c_int;
// type DestroyFn = unsafe extern "C" fn();
// type LambdaEntryFn = unsafe extern "C" fn(arguments: *mut c_void, gc_system: *mut c_void) -> *mut c_void;
