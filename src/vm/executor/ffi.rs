use std::collections::HashMap;
use std::ffi::{c_double, c_void, CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::{Arc, RwLock};

use crate::vm::gc::gc::{GCRef, GCSystem};

/// 定义Lambda函数的类型
pub type CLambdaEntryFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
/// 定义销毁函数的类型
pub type CLambdaDestroyFn = unsafe extern "C" fn();
/// 定义入口
pub type CLambdaBodyFn = unsafe extern "C" fn(
    *mut c_void, // GCRef
    *mut c_void, // GCSystem
) -> *mut c_void; // GCRef
/// 定义Lookup函数的类型
pub type RustLookupFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;

pub mod vm_ffi {
    use lazy_static::lazy_static;

    use super::*;
    use crate::vm::executor::variable::*;
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
        let name = CStr::from_ptr(name).to_string_lossy().into_owned();
        let lookup_table = LOOKUP_TABLE.read().unwrap();
        if let Some(ptr) = lookup_table.get(&name) {
            return ptr.0;
        }
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

        // 类型检查函数
        register_function("is_vm_int", is_vm_int as *mut c_void);
        register_function("is_vm_float", is_vm_float as *mut c_void);
        register_function("is_vm_string", is_vm_string as *mut c_void);
        register_function("is_vm_boolean", is_vm_boolean as *mut c_void);
        register_function("is_vm_null", is_vm_null as *mut c_void);

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
        register_function("set_vm_value", set_vm_value as *mut c_void);
    }

    // 初始化函数表 - 在程序启动时调用
    pub fn init_lookup_table() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            register_standard_functions();
        });
    }

    pub unsafe extern "C" fn new_vm_int64(v: c_longlong, gc_system: *mut c_void) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        return gc_system.new_object(VMInt::new(v as i64)).get_reference() as *mut c_void;
    }

    pub unsafe extern "C" fn new_vm_float64(v: c_double, gc_system: *mut c_void) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_system.new_object(VMFloat::new(v)).get_reference() as *mut c_void
    }

    pub unsafe extern "C" fn new_vm_string(
        s: *const c_char,
        gc_system: *mut c_void,
    ) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        let c_str = CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            gc_system
                .new_object(VMString::new(rust_str))
                .get_reference() as *mut c_void
        } else {
            gc_system.new_object(VMString::new("")).get_reference() as *mut c_void
        }
    }

    pub unsafe extern "C" fn new_vm_boolean(b: c_int, gc_system: *mut c_void) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_system.new_object(VMBoolean::new(b != 0)).get_reference() as *mut c_void
    }

    pub unsafe extern "C" fn new_vm_null(gc_system: *mut c_void) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_system.new_object(VMNull::new()).get_reference() as *mut c_void
    }

    pub unsafe extern "C" fn new_vm_bytes(
        data: *const u8,
        len: c_int,
        gc_system: *mut c_void,
    ) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);

        if data.is_null() || len <= 0 {
            return gc_system
                .new_object(VMBytes::new(&Vec::new()))
                .get_reference() as *mut c_void;
        }

        let bytes = std::slice::from_raw_parts(data, len as usize).to_vec();
        gc_system.new_object(VMBytes::new(&bytes)).get_reference() as *mut c_void
    }

    pub unsafe extern "C" fn new_vm_tuple(gc_system: *mut c_void) -> *mut c_void {
        let gc_system = &mut *(gc_system as *mut GCSystem);
        gc_system
            .new_object(VMTuple::new(&mut Vec::new()))
            .get_reference() as *mut c_void
    }

    // 类型检查函数
    pub unsafe extern "C" fn is_vm_int(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMInt>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_float(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMFloat>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_string(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMString>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_boolean(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMBoolean>() {
            1
        } else {
            0
        }
    }

    pub unsafe extern "C" fn is_vm_null(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 1;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMNull>() {
            1
        } else {
            0
        }
    }

    // 值获取函数
    pub unsafe extern "C" fn get_vm_int_value(obj: *mut c_void) -> std::ffi::c_longlong {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMInt>() {
            gc_ref.as_const_type::<VMInt>().value
        } else {
            0
        }
    }

    pub unsafe extern "C" fn get_vm_float_value(obj: *mut c_void) -> c_double {
        if obj.is_null() {
            return 0.0;
        }
        let gc_ref = &*(obj as *const GCRef);
        if gc_ref.isinstance::<VMFloat>() {
            gc_ref.as_const_type::<VMFloat>().value
        } else if gc_ref.isinstance::<VMInt>() {
            gc_ref.as_const_type::<VMInt>().value as c_double
        } else {
            0.0
        }
    }

    pub unsafe extern "C" fn get_vm_string_value(obj: *mut c_void) -> *mut c_char {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        let gc_ref = &*(obj as *const GCRef);
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

    pub unsafe extern "C" fn get_vm_boolean_value(obj: *mut c_void) -> c_int {
        if obj.is_null() {
            return 0;
        }
        let gc_ref = &*(obj as *const GCRef);
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
    pub unsafe extern "C" fn vm_tuple_append(tuple: *mut c_void, value: *mut c_void) -> c_int {
        if tuple.is_null() || value.is_null() {
            return 0;
        }

        let tuple_ref = &mut *(tuple as *mut GCRef);
        let value_ref = &mut *(value as *mut GCRef);

        if !tuple_ref.isinstance::<VMTuple>() {
            return 0;
        }

        match tuple_ref.as_type::<VMTuple>().append(value_ref) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }

    pub unsafe extern "C" fn vm_tuple_get(
        tuple: *mut c_void,
        index: c_int,
        gc_system: *mut c_void,
    ) -> *mut c_void {
        if tuple.is_null() {
            return std::ptr::null_mut();
        }

        let tuple_ref = &mut *(tuple as *mut GCRef);
        let gc_system = &mut *(gc_system as *mut GCSystem);

        if !tuple_ref.isinstance::<VMTuple>() {
            return std::ptr::null_mut();
        }

        let index_obj = gc_system.new_object(VMInt::new(index as i64));

        match crate::vm::executor::variable::try_index_of_as_vmobject(
            tuple_ref, &index_obj, gc_system,
        ) {
            Ok(value) => value.get_reference() as *mut c_void,
            Err(_) => std::ptr::null_mut(),
        }
    }

    // 对象操作函数
    pub unsafe extern "C" fn get_vm_value(obj: *mut c_void) -> *mut c_void {
        if obj.is_null() {
            return std::ptr::null_mut();
        }

        let gc_ref = &mut *(obj as *mut GCRef);

        // 尝试获取包装类型的值
        if gc_ref.isinstance::<VMWrapper>()
            || gc_ref.isinstance::<VMKeyVal>()
            || gc_ref.isinstance::<VMNamed>()
        {
            if let Ok(value_ref) = crate::vm::executor::variable::try_value_of_as_vmobject(gc_ref) {
                return value_ref.get_reference() as *mut c_void;
            }
        }

        // 如果不是包装类型或获取失败，返回原对象
        obj
    }

    pub unsafe extern "C" fn set_vm_value(target: *mut c_void, value: *mut c_void) -> c_int {
        if target.is_null() || value.is_null() {
            return 0;
        }

        let target_ref = &mut *(target as *mut GCRef);
        let value_ref = &mut *(value as *mut GCRef);

        match crate::vm::executor::variable::try_assign_as_vmobject(target_ref, value_ref) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

pub mod vm_clambda_loading {
    use std::{os::raw::c_void, sync::Arc};

    use libloading::{Library, Symbol};

    use crate::vm::gc::gc::{GCRef, GCSystem};

    use super::{vm_ffi, CLambdaBodyFn, CLambdaDestroyFn, CLambdaEntryFn};

    #[derive(Debug)]
    pub struct CLambda {
        pub clambda_path: String,
        pub lib: Arc<Box<Library>>,
        pub clambda_entry: CLambdaEntryFn,
        pub clambda_destroy: CLambdaDestroyFn,
    }

    pub unsafe fn load_clambda(lib_path: &str) -> Result<CLambda, String> {
        // Load the library
        let lib = Library::new(lib_path).map_err(|e| format!("Failed to load library: {}", e))?;
        let lib_arc = Arc::new(Box::new(lib));

        // Get symbols from the reference to the library
        let clambda_entry: Symbol<CLambdaEntryFn> = lib_arc
            .as_ref()
            .get(b"clambda_entry")
            .map_err(|e| format!("Failed to load clambda_entry: {}", e))?;
        let clambda_destroy: Symbol<CLambdaDestroyFn> = lib_arc
            .as_ref()
            .get(b"clambda_destroy")
            .map_err(|e| format!("Failed to load clambda_destroy: {}", e))?;

        // Create a new CLambda instance
        let clambda = CLambda {
            clambda_path: lib_path.to_string(),
            lib: lib_arc.clone(),
            clambda_entry: *clambda_entry,
            clambda_destroy: *clambda_destroy,
        };

        Ok(clambda)
    }

    pub unsafe fn init_clambda(clambda: &CLambda) {
        // Call the clambda entry point
        (clambda.clambda_entry)(vm_ffi::rust_lookup as *mut c_void);
    }

    pub unsafe fn destroy_clambda(clambda: &CLambda) {
        // Call the clambda destroy function
        (clambda.clambda_destroy)();
    }

    pub unsafe fn call_clambda(
        clambda: &CLambda,
        signature: &String,
        gc_ref: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, String> {
        // 通过签名获取函数指针
        let symbol_name = format!("clambda_{}", signature);
        let func: Symbol<CLambdaBodyFn> = match clambda.lib.as_ref().get(symbol_name.as_bytes()) {
            Ok(func) => func,
            Err(e) => {
                return Err(format!(
                    "Failed to find function with signature '{}': {}",
                    signature, e
                ))
            }
        };

        // 准备参数
        let args_ptr = gc_ref as *mut GCRef as *mut c_void;
        let gc_system_ptr = gc_system as *mut GCSystem as *mut c_void;

        // 调用函数
        let result_ptr = func(args_ptr, gc_system_ptr);

        if result_ptr.is_null() {
            return Err(format!("CLambda function '{}' returned null", signature));
        }

        // 将结果转换为GCRef
        let result_ref = &*(result_ptr as *const GCRef);
        Ok(result_ref.clone())
    }
}
