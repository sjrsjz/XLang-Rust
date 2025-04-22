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

    use crate::executor::variable::{VMLambda, VMLambdaBody};
    use crate::executor::vm::VMError;
    use rustc_hash::FxHashMap as HashMap;
    use rustc_hash::FxHashSet as HashSet;
    use serde_json::Value as JsonValue;

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
        }  else if value.isinstance::<VMTuple>() {
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

    fn check_if_tuple(tuple: GCRef) -> Result<(), VMVariableError> {
        if !tuple.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                tuple,
                "native function's input must be a tuple".to_string(),
            ));
        }
        Ok(())
    }

    pub fn inject_builtin_functions(
        context: &mut Context,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        let mut built_in_functions: HashMap<String, GCRef> = HashMap::default();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "print".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::print".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::print),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "len".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::len".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::len),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "int".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::int".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::to_int),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "float".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::float".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::to_float),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "string".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::string".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::to_string),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "bool".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::bool".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::to_bool),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "bytes".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::bytes".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::to_bytes),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "input".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::input".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::input),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "load_clambda".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::load_clambda".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::load_clambda),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "json_encode".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::json_encode".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::json_encode),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        let mut params = gc_system.new_object(VMTuple::new(&mut vec![]));
        let mut result = gc_system.new_object(VMNull::new());
        built_in_functions.insert(
            "json_decode".to_string(),
            gc_system.new_object(VMLambda::new(
                0,
                "<builtins>::json_decode".to_string(),
                &mut params,
                None,
                None,
                &mut VMLambdaBody::VMNativeFunction(self::json_decode),
                &mut result,
            )),
        );
        params.drop_ref();
        result.drop_ref();

        for (name, func) in built_in_functions.iter_mut() {
            let result = context.let_var(name, func, gc_system);
            func.drop_ref();
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
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
