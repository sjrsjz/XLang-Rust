use rustc_hash::FxHashMap;

use crate::gc::{GCRef, GCSystem};

use super::variable::{VMKeyVal, VMLambda, VMLambdaBody, VMNull, VMString, VMTuple, VMVariableError};

fn check_if_tuple(tuple: GCRef) -> Result<(), VMVariableError> {
    if !tuple.isinstance::<VMTuple>() {
        return Err(VMVariableError::TypeError(
            tuple,
            "native function's input must be a tuple".to_string(),
        ));
    }
    Ok(())
}
// Helper function to create a native VMLambda
fn create_native_lambda(
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
fn build_module(
    functions: &FxHashMap<String, for<'a> fn(GCRef, &'a mut GCSystem) -> Result<GCRef, VMVariableError>>,
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

mod string_utils {
    use rustc_hash::FxHashMap;

    use crate::{
        executor::variable::{
            VMBoolean, VMInt, VMString, VMTuple, VMVariableError,
        },
        gc::{GCRef, GCSystem},
    };

    use super::{build_module, check_if_tuple}; // Import necessary items

    // Helper to extract string from the first argument of the tuple
    fn get_self_string(tuple: &GCRef) -> Result<&String, VMVariableError> {
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.is_empty() {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "String method called with no arguments (missing self)".to_string(),
            ));
        }
        let self_obj = &tuple_obj.values[0];
        if !self_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                self_obj.clone(),
                "First argument must be a string".to_string(),
            ));
        }
        Ok(&self_obj.as_const_type::<VMString>().value)
    }

    // string.split(separator, [maxsplit])
    fn split(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let self_str = get_self_string(&args_tuple)?;

        if tuple_obj.values.len() < 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "split requires at least one argument (separator)".to_string(),
            ));
        }

        let sep_obj = &tuple_obj.values[1];
        if !sep_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                sep_obj.clone(),
                "Separator must be a string".to_string(),
            ));
        }
        let separator = &sep_obj.as_const_type::<VMString>().value;

        // Optional maxsplit argument
        let maxsplit: Option<usize> = if tuple_obj.values.len() > 2 {
            let maxsplit_obj = &tuple_obj.values[2];
            if maxsplit_obj.isinstance::<VMInt>() {
                let val = maxsplit_obj.as_const_type::<VMInt>().value;
                if val < 0 {
                    None // Negative maxsplit means split all
                } else {
                    Some(val as usize)
                }
            } else {
                return Err(VMVariableError::TypeError(
                    maxsplit_obj.clone(),
                    "maxsplit must be an integer".to_string(),
                ));
            }
        } else {
            None // Default: split all occurrences
        };

        let mut result_elements = Vec::new();
        let mut temp_refs = Vec::new();

        let parts: Vec<&str> = match maxsplit {
            Some(n) => self_str.splitn(n + 1, separator).collect(),
            None => self_str.split(separator).collect(),
        };

        for part in parts {
            let vm_part = gc_system.new_object(VMString::new(part));
            result_elements.push(vm_part.clone()); // Clone for the final tuple
            temp_refs.push(vm_part); // Track for dropping later
        }

        let mut element_refs: Vec<&mut GCRef> = result_elements.iter_mut().collect();
        let result_tuple = gc_system.new_object(VMTuple::new(&mut element_refs));

        // Drop refs owned by the new tuple and temporary refs
        for mut r in temp_refs {
            r.drop_ref();
        }
        for mut r in result_elements {
            r.drop_ref();
        }

        Ok(result_tuple)
    }

    // separator.join(iterable)
    fn join(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let separator = get_self_string(&args_tuple)?; // Separator is 'self'

        if tuple_obj.values.len() != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "join requires exactly one argument (iterable)".to_string(),
            ));
        }

        let iterable_obj = &tuple_obj.values[1];
        if !iterable_obj.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                iterable_obj.clone(),
                "Argument to join must be a tuple".to_string(),
            ));
        }
        let iterable_tuple = iterable_obj.as_const_type::<VMTuple>();

        let mut string_parts = Vec::with_capacity(iterable_tuple.values.len());
        for item in &iterable_tuple.values {
            if !item.isinstance::<VMString>() {
                return Err(VMVariableError::TypeError(
                    item.clone(),
                    "All elements in the iterable must be strings for join".to_string(),
                ));
            }
            string_parts.push(item.as_const_type::<VMString>().value.as_str());
        }

        let joined_string = string_parts.join(separator);
        Ok(gc_system.new_object(VMString::new(&joined_string)))
    }

    // string.replace(old, new, [count])
    fn replace(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let self_str = get_self_string(&args_tuple)?;

        if tuple_obj.values.len() < 3 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "replace requires at least two arguments (old, new)".to_string(),
            ));
        }

        let old_obj = &tuple_obj.values[1];
        let new_obj = &tuple_obj.values[2];

        if !old_obj.isinstance::<VMString>() || !new_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(), // Or pinpoint the specific non-string arg
                "Both 'old' and 'new' arguments must be strings".to_string(),
            ));
        }
        let old_str = &old_obj.as_const_type::<VMString>().value;
        let new_str = &new_obj.as_const_type::<VMString>().value;

        // Optional count argument
        let count: Option<usize> = if tuple_obj.values.len() > 3 {
            let count_obj = &tuple_obj.values[3];
            if count_obj.isinstance::<VMInt>() {
                let val = count_obj.as_const_type::<VMInt>().value;
                if val < 0 {
                    None // Negative count means replace all
                } else {
                    Some(val as usize)
                }
            } else {
                return Err(VMVariableError::TypeError(
                    count_obj.clone(),
                    "count must be an integer".to_string(),
                ));
            }
        } else {
            None // Default: replace all occurrences
        };

        let result_string = match count {
            Some(n) => self_str.replacen(old_str, new_str, n),
            None => self_str.replace(old_str, new_str),
        };

        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // string.startswith(prefix)
    fn startswith(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let self_str = get_self_string(&args_tuple)?;

        if tuple_obj.values.len() != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "startswith requires exactly one argument (prefix)".to_string(),
            ));
        }

        let prefix_obj = &tuple_obj.values[1];
        if !prefix_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                prefix_obj.clone(),
                "Prefix must be a string".to_string(),
            ));
        }
        let prefix_str = &prefix_obj.as_const_type::<VMString>().value;

        let result = self_str.starts_with(prefix_str);
        Ok(gc_system.new_object(VMBoolean::new(result)))
    }

    // string.endswith(suffix)
    fn endswith(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let self_str = get_self_string(&args_tuple)?;

        if tuple_obj.values.len() != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "endswith requires exactly one argument (suffix)".to_string(),
            ));
        }

        let suffix_obj = &tuple_obj.values[1];
        if !suffix_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                suffix_obj.clone(),
                "Suffix must be a string".to_string(),
            ));
        }
        let suffix_str = &suffix_obj.as_const_type::<VMString>().value;

        let result = self_str.ends_with(suffix_str);
        Ok(gc_system.new_object(VMBoolean::new(result)))
    }

    // string.strip([chars])
    fn strip(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let self_str = get_self_string(&args_tuple)?;

        let result_string = if tuple_obj.values.len() > 1 {
            let chars_obj = &tuple_obj.values[1];
            if !chars_obj.isinstance::<VMString>() {
                return Err(VMVariableError::TypeError(
                    chars_obj.clone(),
                    "Characters to strip must be a string".to_string(),
                ));
            }
            let chars_str = &chars_obj.as_const_type::<VMString>().value;
            let chars_to_strip: Vec<char> = chars_str.chars().collect();
            self_str.trim_matches(|c| chars_to_strip.contains(&c)).to_string()
        } else {
            // Default: strip whitespace
            self_str.trim().to_string()
        };

        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // string.lower()
    fn lower(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let self_str = get_self_string(&args_tuple)?;
        // Check for extra arguments if desired, though lower usually takes none
        if args_tuple.as_const_type::<VMTuple>().values.len() > 1 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "lower takes no arguments".to_string(),
            ));
        }

        let result_string = self_str.to_lowercase();
        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // string.upper()
    fn upper(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let self_str = get_self_string(&args_tuple)?;
        // Check for extra arguments
        if args_tuple.as_const_type::<VMTuple>().values.len() > 1 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                "upper takes no arguments".to_string(),
            ));
        }

        let result_string = self_str.to_uppercase();
        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // Function to create the string utility module
    pub fn get_string_utils_module(gc_system: &mut GCSystem) -> GCRef {
        let mut functions = FxHashMap::default();
        functions.insert("split".to_string(), split as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("join".to_string(), join as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("replace".to_string(), replace as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("startswith".to_string(), startswith as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("endswith".to_string(), endswith as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("strip".to_string(), strip as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("lower".to_string(), lower as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("upper".to_string(), upper as for<'a> fn(_, &'a mut _) -> _);

        build_module(&functions, gc_system)
    }
}

pub mod native_functions {
    use std::io::Write;

    use crate::{
        executor::{
            context::Context,
            ffi::vm_clambda_loading,
            variable::{
                try_repr_vmobject, try_to_string_vmobject, VMBoolean, VMBytes,
                VMCLambdaInstruction, VMFloat, VMInt, VMKeyVal, VMNull, VMString, VMTuple,
                VMVariableError,
            },
        },
        gc::{GCRef, GCSystem},
    };
    use base64::Engine;

    use crate::executor::vm::VMError;
    use rustc_hash::FxHashSet as HashSet;
    use serde_json::Value as JsonValue;
    use super::string_utils; // Import the new module
    use super::check_if_tuple; // Import the check_if_tuple function

    // Helper function to convert VMObject GCRef to serde_json::Value
    fn vmobject_to_json(
        value: GCRef,
        gc_system: &mut GCSystem,
        visited: &mut HashSet<*const ()>, // 用于检测循环引用
    ) -> Result<JsonValue, VMVariableError> {
        let ptr = value.get_const_reference() as *const ();
        if !visited.insert(ptr) {
            // 检测到循环引用，可以返回 Null 或错误
            // return Err(VMVariableError::ValueError(value, "Circular reference detected during JSON encoding".to_string()));
            return Ok(JsonValue::Null); // 或者返回 Null
        }

        let result = if value.isinstance::<VMNull>() {
            Ok(JsonValue::Null)
        } else if value.isinstance::<VMBoolean>() {
            Ok(JsonValue::Bool(value.as_const_type::<VMBoolean>().value))
        } else if value.isinstance::<VMInt>() {
            // serde_json::Number can represent i64
            Ok(JsonValue::Number(
                value.as_const_type::<VMInt>().value.into(),
            ))
        } else if value.isinstance::<VMFloat>() {
            // serde_json::Number can represent f64. Handle potential NaN/Infinity.
            let f_val = value.as_const_type::<VMFloat>().value;
            serde_json::Number::from_f64(f_val)
                .map(JsonValue::Number)
                .ok_or_else(|| {
                    VMVariableError::ValueError(
                        value.clone(),
                        "Cannot encode NaN or Infinity in JSON".to_string(),
                    )
                })
        } else if value.isinstance::<VMString>() {
            Ok(JsonValue::String(
                value.as_const_type::<VMString>().value.clone(),
            ))
        } else if value.isinstance::<VMTuple>() {
            // --- Modified VMTuple handling ---
            let tuple = value.as_const_type::<VMTuple>();
            let all_keyval = tuple.values.iter().all(|v| v.isinstance::<VMKeyVal>());

            if all_keyval && !tuple.values.is_empty() {
                // Treat as dictionary if all elements are KeyVal
                let mut json_map = serde_json::Map::new();
                for item_ref in &tuple.values {
                    let kv = item_ref.as_const_type::<VMKeyVal>();
                    let key_obj = kv.get_const_key();
                    let val_obj = kv.get_const_value();

                    // Key must be convertible to a JSON string
                    let json_key_val = vmobject_to_json(key_obj.clone(), gc_system, visited)?;
                    let key_str = match json_key_val {
                        JsonValue::String(s) => s,
                        // Optionally handle other key types or return error
                        _ => {
                            // Clean up visited entry before returning error
                            visited.remove(&ptr);
                            return Err(VMVariableError::TypeError(
                                key_obj.clone(),
                                "JSON object keys must be strings".to_string(),
                            ));
                        }
                    };

                    let json_val = vmobject_to_json(val_obj.clone(), gc_system, visited)?;
                    json_map.insert(key_str, json_val);
                }
                Ok(JsonValue::Object(json_map))
            } else {
                // Treat as array otherwise (including empty tuple)
                let mut json_array = Vec::with_capacity(tuple.values.len());
                for item in &tuple.values {
                    json_array.push(vmobject_to_json(item.clone(), gc_system, visited)?);
                }
                Ok(JsonValue::Array(json_array))
            }
            // --- End of Modified VMTuple handling ---
        } else if value.isinstance::<VMKeyVal>() {
            // Represent KeyVal as a two-element array [key, value] if encountered outside a tuple context
            // This might be less common if your primary map representation is Tuple<KeyVal>
            let kv = value.as_const_type::<VMKeyVal>();
            let json_key = vmobject_to_json(kv.get_const_key().clone(), gc_system, visited)?;
            let json_val = vmobject_to_json(kv.get_const_value().clone(), gc_system, visited)?;
            Ok(JsonValue::Array(vec![json_key, json_val]))
        } else if value.isinstance::<VMBytes>() {
            // Encode bytes as Base64 string
            let bytes_val = &value.as_const_type::<VMBytes>().value;
            Ok(JsonValue::String(
                base64::engine::general_purpose::STANDARD.encode(bytes_val),
            ))
        }
        // Add other types if needed (e.g., VMSet, VMNamed) or return error for unsupported types
        else {
            Err(VMVariableError::TypeError(
                value.clone(),
                format!(
                    "Type '{}' cannot be directly encoded to JSON",
                    try_repr_vmobject(value.clone(), None).unwrap_or("?".to_string())
                ),
            ))
        };

        visited.remove(&ptr); // 回溯时移除
        result
    }

    // Helper function to convert serde_json::Value to VMObject GCRef
    fn json_to_vmobject(
        value: JsonValue,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMVariableError> {
        match value {
            JsonValue::Null => Ok(gc_system.new_object(VMNull::new())),
            JsonValue::Bool(b) => Ok(gc_system.new_object(VMBoolean::new(b))),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(gc_system.new_object(VMInt::new(i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(gc_system.new_object(VMFloat::new(f)))
                } else {
                    // Should not happen with standard JSON numbers but handle defensively
                    Err(VMVariableError::ValueError(
                        gc_system.new_object(VMNull::new()), // Placeholder GCRef for error
                        "Invalid JSON number type".to_string(),
                    ))
                }
            }
            JsonValue::String(s) => Ok(gc_system.new_object(VMString::new(&s))),
            JsonValue::Array(arr) => {
                let mut temp_refs = Vec::with_capacity(arr.len());
                for item in arr {
                    let vm_item = json_to_vmobject(item, gc_system)?;
                    temp_refs.push(vm_item);
                }

                // Create a vector of mutable references for VMTuple::new
                let mut vm_elements: Vec<&mut GCRef> = temp_refs.iter_mut().collect();

                // Create the tuple with references to our temp_refs
                let tuple = gc_system.new_object(VMTuple::new(&mut vm_elements));

                // Drop all temp_refs as they've been used by VMTuple::new
                for mut r in temp_refs {
                    r.drop_ref();
                }
                Ok(tuple)
            }
            JsonValue::Object(obj) => {
                // Represent JSON object as a VMTuple of VMKeyVal pairs
                let mut kv_refs = Vec::with_capacity(obj.len());
                let mut temp_refs = Vec::new(); // Manage temporary refs

                for (k, v) in obj {
                    let vm_key = gc_system.new_object(VMString::new(&k));
                    temp_refs.push(vm_key.clone()); // Track key ref

                    let vm_value = json_to_vmobject(v, gc_system)?;
                    temp_refs.push(vm_value.clone()); // Track value ref

                    let mut key_ref = vm_key.clone(); // Ref for VMKeyVal::new
                    let mut val_ref = vm_value.clone(); // Ref for VMKeyVal::new

                    let kv_pair = gc_system.new_object(VMKeyVal::new(&mut key_ref, &mut val_ref));
                    kv_refs.push(kv_pair);
                    // Don't add to temp_refs as we're keeping these references
                }

                // Create a mutable slice of mutable references for VMTuple::new
                let mut kv_pairs: Vec<&mut GCRef> = kv_refs.iter_mut().collect();

                let tuple = gc_system.new_object(VMTuple::new(&mut kv_pairs));
                // Drop all temporary refs created in this scope
                for mut r in temp_refs {
                    r.drop_ref();
                }
                // Now drop the kv_refs as they've been used by VMTuple::new
                for mut r in kv_refs {
                    r.drop_ref();
                }
                Ok(tuple)
            }
        }
    }

    pub fn inject_builtin_functions(
        context: &mut Context,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        let built_in_functions: [(
            &str,
            fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
        ); 11] = [
            ("print", self::print),
            ("len", self::len),
            ("int", self::to_int),
            ("float", self::to_float),
            ("string", self::to_string),
            ("bool", self::to_bool),
            ("bytes", self::to_bytes),
            ("input", self::input),
            ("load_clambda", self::load_clambda),
            ("json_encode", self::json_encode),
            ("json_decode", self::json_decode),
        ];

        for (name, func_ptr) in built_in_functions.iter() {
            let mut lambda_ref = super::create_native_lambda(name, *func_ptr, gc_system)
                .map_err(|e| VMError::VMVariableError(e))?; // Handle potential errors during lambda creation

            let result = context.let_var(name, &mut lambda_ref, gc_system);
            lambda_ref.drop_ref(); // Drop the ref created by create_native_lambda

            if let Err(context_error) = result {
                // If let_var fails, we might want to clean up already added functions,
                // but for simplicity, we just return the error here.
                return Err(VMError::ContextError(context_error));
            }
        }

        // Inject the string module
        let mut string_module = string_utils::get_string_utils_module(gc_system);
        let result = context.let_var("string_utils", &mut string_module, gc_system);
        string_module.drop_ref();

        if let Err(context_error) = result {
            return Err(VMError::ContextError(context_error));
        }

        Ok(())
    }

    pub fn json_encode(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "json_encode function takes exactly one argument".to_string(),
            ));
        }
        let object_to_encode = tuple_obj.values[0].clone();

        let mut visited = HashSet::default();
        let json_value = vmobject_to_json(object_to_encode, gc_system, &mut visited)?;

        match serde_json::to_string(&json_value) {
            Ok(json_string) => Ok(gc_system.new_object(VMString::new(&json_string))),
            Err(e) => Err(VMVariableError::ValueError(
                tuple.clone(), // Or maybe the object_to_encode?
                format!("Failed to serialize to JSON: {}", e),
            )),
        }
    }

    pub fn json_decode(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "json_decode function takes exactly one argument".to_string(),
            ));
        }

        let json_string_obj = tuple_obj.values[0].clone();
        if !json_string_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                json_string_obj,
                "Argument to json_decode must be a string".to_string(),
            ));
        }

        let json_string = &json_string_obj.as_const_type::<VMString>().value;

        match serde_json::from_str::<JsonValue>(json_string) {
            Ok(parsed_json) => json_to_vmobject(parsed_json, gc_system),
            Err(e) => Err(VMVariableError::ValueError(
                json_string_obj, // Keep the original string object for the error context
                format!("Failed to parse JSON string: {}", e),
            )),
        }
    }

    pub fn print(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple = tuple.as_const_type::<VMTuple>();
        let mut result = String::new();
        for obj in &tuple.values {
            let repr = try_to_string_vmobject(obj.clone(), None)?;
            result.push_str(&format!("{} ", repr));
        }
        result = result.trim_end_matches(" ").to_string();
        println!("{}", result);
        let obj = gc_system.new_object(VMNull::new());
        Ok(obj)
    }

    pub fn len(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "len function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMTuple>() {
            let inner_tuple = tuple_obj.values[0].as_const_type::<VMTuple>();
            let obj = gc_system.new_object(VMInt::new(inner_tuple.values.len() as i64));
            Ok(obj)
        } else if tuple_obj.values[0].isinstance::<VMString>() {
            let inner_string = tuple_obj.values[0].as_const_type::<VMString>();
            let obj = gc_system.new_object(VMInt::new(inner_string.value.len() as i64));
            return Ok(obj);
        } else if tuple_obj.values[0].isinstance::<VMBytes>() {
            let inner_bytes = tuple_obj.values[0].as_const_type::<VMBytes>();
            let obj = gc_system.new_object(VMInt::new(inner_bytes.value.len() as i64));
            return Ok(obj);
        } else {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "len function's input should be a string or a tuple".to_string(),
            ));
        }
    }

    pub fn to_int(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_int function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMInt::new(0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_const_type::<VMBoolean>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_int function's input should be a int-able value".to_string(),
        ))
    }

    pub fn to_float(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_float function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMFloat::new(0.0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMBoolean>()
                .to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_float function's input should be a float-able value".to_string(),
        ))
    }

    pub fn to_string(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_string function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .to_string()?;
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMString::new("null")));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMBoolean>()
                .to_string()?;
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        if tuple_obj.values[0].isinstance::<VMBytes>() {
            let data = tuple_obj.values[0].as_const_type::<VMBytes>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_string function's input should be a string-able value".to_string(),
        ))
    }

    pub fn to_bool(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_bool function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMBoolean::new(false)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_const_type::<VMBoolean>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_bool function's input should be a bool-able value".to_string(),
        ))
    }
    pub fn to_bytes(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_bytes function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMBytes>() {
            return Ok(gc_system.new_object(VMBytes::new(
                &tuple_obj.values[0].as_const_type::<VMBytes>().value,
            )));
        } else if tuple_obj.values[0].isinstance::<VMString>() {
            // 将字符串转换为字节序列
            let string_value = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .value
                .clone();
            return Ok(gc_system.new_object(VMBytes::new(&string_value.as_bytes().to_vec())));
        } else if tuple_obj.values[0].isinstance::<VMInt>() {
            // 支持单字节的整数转字节
            let int_value = tuple_obj.values[0].as_const_type::<VMInt>().value;
            if !(0..=255).contains(&int_value) {
                return Err(VMVariableError::ValueError(
                    tuple_obj.values[0].clone(),
                    "Integer values for bytes conversion must be between 0 and 255".to_string(),
                ));
            }
            return Ok(gc_system.new_object(VMBytes::new(&vec![int_value as u8])));
        } else if tuple_obj.values[0].isinstance::<VMTuple>() {
            // 支持整数元组转字节序列
            let inner_tuple = tuple_obj.values[0].as_const_type::<VMTuple>();
            let mut byte_vec = Vec::with_capacity(inner_tuple.values.len());

            for value in &inner_tuple.values {
                if !value.isinstance::<VMInt>() {
                    return Err(VMVariableError::ValueError(
                        value.clone(),
                        "All elements in tuple must be integers for bytes conversion".to_string(),
                    ));
                }

                let int_value = value.as_const_type::<VMInt>().value;
                if !(0..=255).contains(&int_value) {
                    return Err(VMVariableError::ValueError(
                        value.clone(),
                        "Integer values for bytes conversion must be between 0 and 255".to_string(),
                    ));
                }

                byte_vec.push(int_value as u8);
            }

            return Ok(gc_system.new_object(VMBytes::new(&byte_vec)));
        }

        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_bytes function's input should be a bytes, string, integer, or tuple of integers"
                .to_string(),
        ))
    }

    pub fn input(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "input function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .to_string()?;
            print!("{} ", data);
            std::io::stdout().flush().unwrap_or(());
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let data = input.trim().to_string();
            return Ok(gc_system.new_object(VMString::new(&data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "input function's input should be a string".to_string(),
        ))
    }

    pub fn load_clambda(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "load_clambda function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .to_string()?;
            let mut clambda = unsafe {
                vm_clambda_loading::load_clambda(&data).map_err(|e| {
                    VMVariableError::ValueError(
                        tuple.clone(),
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
            tuple.clone(),
            "load_clambda function's input should be a string".to_string(),
        ))
    }
}
