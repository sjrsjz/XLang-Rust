mod string_utils;
mod fs;
mod io;
mod types;
mod serialization;
mod async_request;

use rustc_hash::FxHashMap;
use xlang_vm_core::executor::context::Context;
use xlang_vm_core::executor::ffi::vm_clambda_loading;
use xlang_vm_core::executor::vm::VMError;
use xlang_vm_core::gc::{GCRef, GCSystem};
use xlang_vm_core::executor::variable::{VMCLambdaInstruction, VMKeyVal, VMLambda, VMLambdaBody, VMNull, VMString, VMTuple, VMVariableError};
pub(crate) fn check_if_tuple(tuple: GCRef) -> Result<(), VMVariableError> {
    if !tuple.isinstance::<VMTuple>() {
        return Err(VMVariableError::TypeError(
            tuple,
            "native function's input must be a tuple".to_string(),
        ));
    }
    Ok(())
}
// Helper function to create a native VMLambda
pub(crate) fn create_native_lambda(
    name: &str,
    native_fn: fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    // Create empty tuple for default args (can be shared or created anew)
    // Using a shared empty tuple might be slightly more efficient if possible,
    // but creating anew is safer and simpler here.
    let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
    let mut result = gc_system.new_object(VMNull::new()); // Default result placeholder

    let lambda = gc_system.new_object(VMLambda::new(
        0,                               // code_position, 0 for native
        format!("<builtins>::{}", name), // signature
        &mut params,
        None, // capture
        None, // self_object
        &mut VMLambdaBody::VMNativeFunction(native_fn),
        &mut result,
    ));

    // Drop refs owned by the lambda now
    params.drop_ref();
    result.drop_ref();

    Ok(lambda)
}

// Helper function to build a module tuple from a map of functions
pub(crate) fn build_module(
    functions: &FxHashMap<&str, for<'a> fn(GCRef, &'a mut GCSystem) -> Result<GCRef, VMVariableError>>,
    gc_system: &mut GCSystem,
) -> GCRef {
    let mut module = gc_system.new_object(VMTuple::new(&mut vec![]));
    for (name, func) in functions {
        let mut func_ref = create_native_lambda(name, *func, gc_system).unwrap();
        let mut key = gc_system.new_object(VMString::new(name));
        let mut kv_pair = gc_system.new_object(VMKeyVal::new(&mut key, &mut func_ref));
        let _ = module.as_type::<VMTuple>().append(&mut kv_pair);
        func_ref.drop_ref(); // Drop the ref created by create_native_lambda
        key.drop_ref(); // Drop the ref created by VMString::new
        kv_pair.drop_ref(); // Drop the ref created by VMKeyVal::new
    }
    module
}


pub fn load_clambda(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_const_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone(),
            format!("load_clambda expected 1 argument, got {}", tuple_obj.values.len()),
        ));
    }
    let target_obj = &tuple_obj.values[0];
    if target_obj.isinstance::<VMString>() {
        let data = target_obj
            .as_const_type::<VMString>()
            .to_string()?;
        let mut clambda = unsafe {
            vm_clambda_loading::load_clambda(&data).map_err(|e| {
                VMVariableError::ValueError(
                    target_obj.clone(), // Error points to the specific object
                    format!("Failed to load clambda: {}", e),
                )
            })?
        };
        unsafe {
            vm_clambda_loading::init_clambda(&mut clambda);
        }
        let vm_clambda = gc_system.new_object(VMCLambdaInstruction::new(clambda));
        return Ok(vm_clambda);
    }
    Err(VMVariableError::TypeError(
        target_obj.clone(), // Error points to the specific object
        "Argument for load_clambda must be a string (path)".to_string(),
    ))
}

pub fn inject_builtin_functions(
    context: &mut Context,
    gc_system: &mut GCSystem,
) -> Result<(), VMError> {
    let fs = fs::get_fs_module();
    let fs_map = fs.into_iter().collect::<FxHashMap<_, _>>();
    let fs_module = build_module(&fs_map, gc_system);

    let io = io::get_io_functions();
    let io_map = io.into_iter().collect::<FxHashMap<_, _>>();
    let io_module = build_module(&io_map, gc_system);

    let types = types::get_type_conversion_functions();
    let types_map = types.into_iter().collect::<FxHashMap<_, _>>();
    let types_module = build_module(&types_map, gc_system);

    let serialization = serialization::get_serialization_functions();
    let serialization_map = serialization.into_iter().collect::<FxHashMap<_, _>>();
    let serialization_module = build_module(&serialization_map, gc_system);

    let string_utils = string_utils::get_string_utils_module();
    let string_utils_map = string_utils.into_iter().collect::<FxHashMap<_, _>>();
    let string_utils_module = build_module(&string_utils_map, gc_system);

    let async_request = async_request::get_request_functions();
    let async_request_map = async_request.into_iter().collect::<FxHashMap<_, _>>();
    let async_request_module = build_module(&async_request_map, gc_system);

    let mut builtins_map = FxHashMap::default();
    builtins_map.insert("fs", fs_module);
    builtins_map.insert("io", io_module);
    builtins_map.insert("types", types_module);
    builtins_map.insert("serialization", serialization_module);
    builtins_map.insert("string_utils", string_utils_module);
    builtins_map.insert("async_request", async_request_module);

    for (name, module) in &mut builtins_map {
        context.let_var(name,  module, gc_system)
            .map_err(|e| VMError::ContextError(e))?;
        module.drop_ref(); // Drop the ref created by build_module
    }

    // 构建 load_clambda 函数
    let mut load_clambda_ref = create_native_lambda("load_clambda", load_clambda, gc_system).unwrap();
    context
        .let_var("load_clambda", &mut load_clambda_ref, gc_system)
        .map_err(|e| VMError::ContextError(e))?;
    load_clambda_ref.drop_ref(); // Drop the ref created by create_native_lambda

    Ok(())

}