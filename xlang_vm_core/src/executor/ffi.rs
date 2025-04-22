use std::collections::HashMap;
use std::ffi::{CStr, CString, c_double, c_void};
use std::os::raw::{c_char, c_int};
use std::sync::{Arc, RwLock};

use crate::gc::gc::{GCObject, GCRef, GCSystem};

/// 定义Lambda函数的类型
pub type CLambdaEntryFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
/// 定义销毁函数的类型
pub type CLambdaDestroyFn = unsafe extern "C" fn();
/// 定义入口
pub type CLambdaBodyFn = unsafe extern "C" fn(
    FFIGCRef,    // GCRef
    *mut c_void, // GCSystem
) -> FFIGCRef; // GCRef

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FFIGCRef {
    pub data: *mut c_void,   // 指向对象数据的指针
    pub vtable: *mut c_void, // 指向类型虚表的指针
}

impl FFIGCRef {
    pub fn is_null(&self) -> bool {
        self.data.is_null()
    }
}

// 将GCRef转换为FFIGCRef
pub fn gc_ref_to_ffi(gc_ref: &GCRef) -> FFIGCRef {
    let fat_ptr = gc_ref.get_reference();

    // 安全地将胖指针拆分为两部分
    let (data, vtable) = unsafe {
        let ptr_parts = std::mem::transmute::<*mut dyn GCObject, [*mut c_void; 2]>(fat_ptr);
        (ptr_parts[0], ptr_parts[1])
    };

    FFIGCRef { data, vtable }
}

// 将FFIGCRef转换为GCRef - 使用transmute进行反向转换
pub fn ffi_to_gc_ref(ffi_ref: &FFIGCRef) -> GCRef {
    let fat_ptr = unsafe {
        // 将两个指针重新组合为一个胖指针数组
        let ptr_parts: [*mut c_void; 2] = [ffi_ref.data, ffi_ref.vtable];
        // 然后将这个数组转换回胖指针
        std::mem::transmute::<[*mut c_void; 2], *mut dyn GCObject>(ptr_parts)
    };

    GCRef::new(fat_ptr)
}

pub mod vm_ffi {
    use lazy_static::lazy_static;

    use super::*;
    use crate::executor::variable::*;
    use std::ffi::c_longlong;

    // A wrapper around *mut c_void that implements Send and Sync
    struct ThreadSafePtr(*mut c_void);
    // This is safe because we're controlling access through RwLock
    unsafe impl Send for ThreadSafePtr {}
    unsafe impl Sync for ThreadSafePtr {}
    lazy_static! {
        static ref LOOKUP_TABLE: Arc<RwLock<HashMap<String, ThreadSafePtr>>> =
            Arc::new(RwLock::new(HashMap::new()));
    }

    // 注册函数到查找表
    pub fn register_function(name: &str, func_ptr: *mut c_void) {
        let mut lookup_table = LOOKUP_TABLE.write().unwrap();
        lookup_table.insert(name.to_string(), ThreadSafePtr(func_ptr));
    }

    pub unsafe extern "C" fn rust_lookup(name: *const c_char) -> *mut c_void {
        init_lookup_table();
        if name.is_null() {
            return std::ptr::null_mut();
        }
        let name = CStr::from_ptr(name).to_string_lossy().into_owned();
        let lookup_table = LOOKUP_TABLE.read().unwrap();
        if let Some(ptr) = lookup_table.get(&name) {
            return ptr.0;
        }
        println!("Function not found: {}", name);
        std::ptr::null_mut()
    }

    // 批量注册标准函数
    pub fn register_standard_functions() {
        // 类型创建函数
        register_function("new_vm_int64", new_vm_int64 as *mut c_void);
        register_function("new_vm_float64", new_vm_float64 as *mut c_void);
        register_function("new_vm_string", new_vm_string as *mut c_void);
        register_function("new_vm_boolean", new_vm_boolean as *mut c_void);
        register_function("new_vm_null", new_vm_null as *mut c_void);
        register_function("new_vm_bytes", new_vm_bytes as *mut c_void);
        register_function("new_vm_tuple", new_vm_tuple as *mut c_void);
        register_function("new_vm_keyval", new_vm_keyval as *mut c_void);
        register_function("new_vm_named", new_vm_named as *mut c_void);
        register_function("new_vm_wrapper", new_vm_wrapper as *mut c_void);

        // 类型检查函数
        register_function("is_vm_int", is_vm_int as *mut c_void);
        register_function("is_vm_float", is_vm_float as *mut c_void);
        register_function("is_vm_string", is_vm_string as *mut c_void);
        register_function("is_vm_boolean", is_vm_boolean as *mut c_void);
        register_function("is_vm_null", is_vm_null as *mut c_void);
        register_function("is_vm_bytes", is_vm_bytes as *mut c_void);
        register_function("is_vm_tuple", is_vm_tuple as *mut c_void);
        register_function("is_vm_keyval", is_vm_keyval as *mut c_void);
        register_function("is_vm_named", is_vm_named as *mut c_void);
        register_function("is_vm_wrapper", is_vm_wrapper as *mut c_void);

        // 值获取函数
        register_function("get_vm_int_value", get_vm_int_value as *mut c_void);
        register_function("get_vm_float_value", get_vm_float_value as *mut c_void);
        register_function("get_vm_string_value", get_vm_string_value as *mut c_void);
        register_function("get_vm_boolean_value", get_vm_boolean_value as *mut c_void);

        // 集合操作函数
        register_function("vm_tuple_append", vm_tuple_append as *mut c_void);
        register_function("vm_tuple_get", vm_tuple_get as *mut c_void);

        // 对象操作函数
        register_function("get_vm_value", get_vm_value as *mut c_void);
        register_function("get_vm_key", get_vm_key as *mut c_void);
        register_function("set_vm_value", set_vm_value as *mut c_void);
        register_function("get_len", get_len as *mut c_void);
        register_function("clone_ref", clone_ref as *mut c_void);
        register_function("drop_ref", drop_ref as *mut c_void);
    }

    // 初始化函数表 - 在程序启动时调用
    pub fn init_lookup_table() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            register_standard_functions();
        });
    }

    pub unsafe extern "C" fn clone_ref(v: FFIGCRef) -> c_int {
        if v.is_null() {
            return 0;
        }
        let mut gc_ref = ffi_to_gc_ref(&v);
        let _ = gc_ref.clone_ref();
        1
    }

    pub unsafe extern "C" fn drop_ref(v: FFIGCRef) -> c_int {
        if v.is_null() {
            return 0;
        }
        let mut gc_ref = ffi_to_gc_ref(&v);
        gc_ref.drop_ref();
        1
    }

    pub unsafe extern "C" fn get_len(v: FFIGCRef) -> c_longlong {
        if v.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&v);
        if gc_ref.isinstance::<VMTuple>() {
            let tuple = gc_ref.as_const_type::<VMTuple>();
            let len = tuple.len() as c_longlong;
            return len;
        }
        if gc_ref.isinstance::<VMBytes>() {
            let bytes = gc_ref.as_const_type::<VMBytes>();
            let len = bytes.len() as c_longlong;
            return len;
        }
        if gc_ref.isinstance::<VMString>() {
            let string = gc_ref.as_const_type::<VMString>();
            let len = string.value.len() as c_longlong;
            return len;
        }
        if gc_ref.isinstance::<VMRange>() {
            let range = gc_ref.as_const_type::<VMRange>();
            let len = range.len() as c_longlong;
            return len;
        }
        0
    }

    pub unsafe extern "C" fn new_vm_int64(v: c_longlong, gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_ref_to_ffi(&gc_system.new_object(VMInt::new(v)))
    }

    pub unsafe extern "C" fn new_vm_float64(v: c_double, gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_ref_to_ffi(&gc_system.new_object(VMFloat::new(v)))
    }

    pub unsafe extern "C" fn new_vm_string(s: *const c_char, gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        let c_str = CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            gc_ref_to_ffi(&gc_system.new_object(VMString::new(rust_str)))
        } else {
            gc_ref_to_ffi(&gc_system.new_object(VMString::new("")))
        }
    }

    pub unsafe extern "C" fn new_vm_boolean(b: c_int, gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_ref_to_ffi(&gc_system.new_object(VMBoolean::new(b != 0)))
    }

    pub unsafe extern "C" fn new_vm_null(gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_ref_to_ffi(&gc_system.new_object(VMNull::new()))
    }

    pub unsafe extern "C" fn new_vm_bytes(
        data: *const u8,
        len: c_int,
        gc_system: *mut c_void,
    ) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);

        if data.is_null() || len <= 0 {
            return gc_ref_to_ffi(&gc_system.new_object(VMBytes::new(&Vec::new())));
        }

        let bytes = std::slice::from_raw_parts(data, len as usize).to_vec();
        gc_ref_to_ffi(&gc_system.new_object(VMBytes::new(&bytes)))
    }

    pub unsafe extern "C" fn new_vm_tuple(gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_ref_to_ffi(&gc_system.new_object(VMTuple::new(&mut Vec::new())))
    }

    pub unsafe extern "C" fn new_vm_keyval(
        key: FFIGCRef,
        value: FFIGCRef,
        gc_system: *mut c_void,
    ) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        let key_ref = &mut ffi_to_gc_ref(&key);
        let value_ref = &mut ffi_to_gc_ref(&value);
        gc_ref_to_ffi(&gc_system.new_object(VMKeyVal::new(key_ref, value_ref)))
    }

    pub unsafe extern "C" fn new_vm_named(
        key: FFIGCRef,
        value: FFIGCRef,
        gc_system: *mut c_void,
    ) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        let key_ref = &mut ffi_to_gc_ref(&key);
        let value_ref = &mut ffi_to_gc_ref(&value);
        gc_ref_to_ffi(&gc_system.new_object(VMNamed::new(key_ref, value_ref)))
    }

    pub unsafe extern "C" fn new_vm_wrapper(value: FFIGCRef, gc_system: *mut c_void) -> FFIGCRef {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        let value_ref = &mut ffi_to_gc_ref(&value);
        gc_ref_to_ffi(&gc_system.new_object(VMWrapper::new(value_ref)))
    }

    // 类型检查函数
    pub unsafe extern "C" fn is_vm_int(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMInt>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_float(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMFloat>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_string(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMString>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_boolean(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMBoolean>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_null(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 1;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMNull>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_bytes(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMBytes>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_tuple(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMTuple>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_keyval(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMKeyVal>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_named(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMNamed>() { 1 } else { 0 }
    }

    pub unsafe extern "C" fn is_vm_wrapper(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMWrapper>() {
            1
        } else {
            0
        }
    }

    // 值获取函数
    pub unsafe extern "C" fn get_vm_int_value(obj: FFIGCRef) -> std::ffi::c_longlong {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMInt>() {
            gc_ref.as_const_type::<VMInt>().value
        } else {
            0
        }
    }

    pub unsafe extern "C" fn get_vm_float_value(obj: FFIGCRef) -> c_double {
        if obj.is_null() {
            return 0.0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMFloat>() {
            gc_ref.as_const_type::<VMFloat>().value
        } else if gc_ref.isinstance::<VMInt>() {
            gc_ref.as_const_type::<VMInt>().value as c_double
        } else {
            0.0
        }
    }

    pub unsafe extern "C" fn get_vm_string_value(obj: FFIGCRef) -> *mut c_char {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMString>() {
            let str_value = &gc_ref.as_const_type::<VMString>().value;
            match CString::new(str_value.clone()) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    }

    pub unsafe extern "C" fn get_vm_boolean_value(obj: FFIGCRef) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = ffi_to_gc_ref(&obj);
        if gc_ref.isinstance::<VMBoolean>() {
            if gc_ref.as_const_type::<VMBoolean>().value {
                1
            } else {
                0
            }
        } else {
            0
        }
    }

    // 集合操作函数
    pub unsafe extern "C" fn vm_tuple_append(tuple: FFIGCRef, value: FFIGCRef) -> c_int {
        if tuple.is_null() || value.is_null() {
            return 0;
        }

        let tuple_ref = &mut ffi_to_gc_ref(&tuple);
        let value_ref = &mut ffi_to_gc_ref(&value);
        if !tuple_ref.isinstance::<VMTuple>() {
            return 0;
        }

        match tuple_ref.as_type::<VMTuple>().append(value_ref) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }

    pub unsafe extern "C" fn vm_tuple_get(
        tuple: FFIGCRef,
        index: c_int,
        gc_system: *mut c_void,
    ) -> FFIGCRef {
        if tuple.is_null() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }

        let tuple_ref = &mut ffi_to_gc_ref(&tuple);
        let gc_system = unsafe { &mut *(gc_system as *mut GCSystem) };

        if !tuple_ref.isinstance::<VMTuple>() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }

        let index_obj = gc_system.new_object(VMInt::new(index as i64));

        match crate::executor::variable::try_index_of_as_vmobject(tuple_ref, &index_obj, gc_system)
        {
            Ok(value) => gc_ref_to_ffi(&value),
            Err(_) => FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            },
        }
    }

    // 对象操作函数
    pub unsafe extern "C" fn get_vm_value(obj: FFIGCRef) -> FFIGCRef {
        if obj.is_null() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }

        let mut gc_ref = ffi_to_gc_ref(&obj);

        let v = try_value_of_as_vmobject(&mut gc_ref);
        if v.is_err() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }
        let value_ref = v.unwrap();
        gc_ref_to_ffi(value_ref)
    }

    pub unsafe extern "C" fn get_vm_key(obj: FFIGCRef) -> FFIGCRef {
        if obj.is_null() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }

        let mut gc_ref = ffi_to_gc_ref(&obj);

        let v = try_key_of_as_vmobject(&mut gc_ref);
        if v.is_err() {
            return FFIGCRef {
                data: std::ptr::null_mut(),
                vtable: std::ptr::null_mut(),
            };
        }
        let value_ref = v.unwrap();
        gc_ref_to_ffi(value_ref)
    }

    pub unsafe extern "C" fn set_vm_value(target: FFIGCRef, value: FFIGCRef) -> c_int {
        if target.is_null() || value.is_null() {
            return 0;
        }

        let target_ref = &mut ffi_to_gc_ref(&target);
        let value_ref = &mut ffi_to_gc_ref(&value);

        match crate::executor::variable::try_assign_as_vmobject(target_ref, value_ref) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

pub mod vm_clambda_loading {
    use std::{os::raw::c_void, sync::Arc};

    use libloading::{Library, Symbol};

    use crate::gc::gc::{GCRef, GCSystem};

    use super::{
        CLambdaBodyFn, CLambdaDestroyFn, CLambdaEntryFn, ffi_to_gc_ref, gc_ref_to_ffi, vm_ffi,
    };

    #[derive(Debug)]
    pub struct CLambda {
        pub clambda_path: String,
        pub lib: Arc<Box<Library>>,
        pub clambda_entry: CLambdaEntryFn,
        pub clambda_destroy: CLambdaDestroyFn,
    }

    impl Clone for CLambda {
        fn clone(&self) -> Self {
            CLambda {
                clambda_path: self.clambda_path.clone(),
                lib: self.lib.clone(),
                clambda_entry: self.clambda_entry,
                clambda_destroy: self.clambda_destroy,
            }
        }
    }

    pub unsafe fn load_clambda(lib_path: &str) -> Result<CLambda, String> {
        // Load the library
        let lib = unsafe {
            Library::new(lib_path).map_err(|e| format!("Failed to load library: {}", e))
        }?;
        let lib_arc = Arc::new(Box::new(lib));

        // Get symbols from the reference to the library
        let clambda_entry: Symbol<CLambdaEntryFn> = unsafe {
            lib_arc
                .as_ref()
                .get(b"clambda_entry")
                .map_err(|e| format!("Failed to load clambda_entry: {}", e))
        }?;
        let clambda_destroy: Symbol<CLambdaDestroyFn> = unsafe {
            lib_arc
                .as_ref()
                .get(b"clambda_destroy")
                .map_err(|e| format!("Failed to load clambda_destroy: {}", e))
        }?;

        // Create a new CLambda instance
        let clambda = CLambda {
            clambda_path: lib_path.to_string(),
            lib: lib_arc.clone(),
            clambda_entry: *clambda_entry,
            clambda_destroy: *clambda_destroy,
        };

        Ok(clambda)
    }

    pub unsafe fn init_clambda(clambda: &mut CLambda) {
        // Call the clambda entry point
        unsafe { (clambda.clambda_entry)(vm_ffi::rust_lookup as *mut c_void) };
    }

    pub unsafe fn destroy_clambda(clambda: &mut CLambda) {
        // Call the clambda destroy function
        unsafe { (clambda.clambda_destroy)() };
    }

    pub unsafe fn call_clambda(
        clambda: &CLambda,
        signature: &String,
        gc_ref: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, String> {
        // 通过签名获取函数指针
        let symbol_name = format!("clambda_{}", signature);
        let func: Symbol<CLambdaBodyFn> =
            match unsafe { clambda.lib.as_ref().get(symbol_name.as_bytes()) } {
                Ok(func) => func,
                Err(e) => {
                    return Err(format!(
                        "Failed to find function with signature '{}': {}",
                        signature, e
                    ));
                }
            };

        // 准备参数
        let args_ptr = gc_ref_to_ffi(gc_ref);
        let gc_system_ptr = gc_system as *mut GCSystem as *mut c_void;

        // 调用函数
        let result_ptr = func(args_ptr, gc_system_ptr);

        if result_ptr.is_null() {
            return Err(format!("CLambda function '{}' returned null", signature));
        }

        // 将结果转换为GCRef
        let result_ref = ffi_to_gc_ref(&result_ptr);
        Ok(result_ref)
    }
}
