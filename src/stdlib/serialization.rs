use xlang_vm_core::{
    executor::variable::{
        try_repr_vmobject, VMBoolean, VMBytes, VMFloat, VMInt, VMKeyVal, VMNull, VMString, VMTuple, VMVariableError
    },
    gc::{GCRef, GCSystem},
};
use base64::Engine;
use rustc_hash::FxHashSet as HashSet;
use serde_json::Value as JsonValue;
// Assuming check_if_tuple will be available via super
use super::check_if_tuple;

// Helper function (remains private to this module)
fn vmobject_to_json(
    value: GCRef,
    gc_system: &mut GCSystem,
    visited: &mut HashSet<*const ()>,
) -> Result<JsonValue, VMVariableError> {
    let ptr = value.get_const_reference() as *const ();
    if !visited.insert(ptr) {
        return Ok(JsonValue::Null); // Handle circular reference
    }

    let result = if value.isinstance::<VMNull>() {
        Ok(JsonValue::Null)
    } else if value.isinstance::<VMBoolean>() {
        Ok(JsonValue::Bool(value.as_const_type::<VMBoolean>().value))
    } else if value.isinstance::<VMInt>() {
        Ok(JsonValue::Number(
            value.as_const_type::<VMInt>().value.into(),
        ))
    } else if value.isinstance::<VMFloat>() {
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
        let tuple = value.as_const_type::<VMTuple>();
        let all_keyval = tuple.values.iter().all(|v| v.isinstance::<VMKeyVal>());

        if all_keyval && !tuple.values.is_empty() {
            let mut json_map = serde_json::Map::new();
            for item_ref in &tuple.values {
                let kv = item_ref.as_const_type::<VMKeyVal>();
                let key_obj = kv.get_const_key();
                let val_obj = kv.get_const_value();
                let json_key_val = vmobject_to_json(key_obj.clone(), gc_system, visited)?;
                let key_str = match json_key_val {
                    JsonValue::String(s) => s,
                    _ => {
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
            let mut json_array = Vec::with_capacity(tuple.values.len());
            for item in &tuple.values {
                json_array.push(vmobject_to_json(item.clone(), gc_system, visited)?);
            }
            Ok(JsonValue::Array(json_array))
        }
    } else if value.isinstance::<VMKeyVal>() {
        let kv = value.as_const_type::<VMKeyVal>();
        let json_key = vmobject_to_json(kv.get_const_key().clone(), gc_system, visited)?;
        let json_val = vmobject_to_json(kv.get_const_value().clone(), gc_system, visited)?;
        Ok(JsonValue::Array(vec![json_key, json_val]))
    } else if value.isinstance::<VMBytes>() {
        let bytes_val = &value.as_const_type::<VMBytes>().value;
        Ok(JsonValue::String(
            base64::engine::general_purpose::STANDARD.encode(bytes_val),
        ))
    } else {
        Err(VMVariableError::TypeError(
            value.clone(),
            format!(
                "Type '{}' cannot be directly encoded to JSON",
                // Use the imported try_repr_vmobject
                try_repr_vmobject(value.clone(), None).unwrap_or("?".to_string())
            ),
        ))
    };

    visited.remove(&ptr);
    result
}

// Helper function (remains private to this module)
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
                Err(VMVariableError::ValueError(
                    gc_system.new_object(VMNull::new()),
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
            let mut vm_elements: Vec<&mut GCRef> = temp_refs.iter_mut().collect();
            let tuple = gc_system.new_object(VMTuple::new(&mut vm_elements));
            for mut r in temp_refs {
                r.drop_ref();
            }
            Ok(tuple)
        }
        JsonValue::Object(obj) => {
            let mut kv_refs = Vec::with_capacity(obj.len());
            let mut temp_refs = Vec::new();
            for (k, v) in obj {
                let vm_key = gc_system.new_object(VMString::new(&k));
                temp_refs.push(vm_key.clone());
                let vm_value = json_to_vmobject(v, gc_system)?;
                temp_refs.push(vm_value.clone());
                let mut key_ref = vm_key.clone();
                let mut val_ref = vm_value.clone();
                let kv_pair = gc_system.new_object(VMKeyVal::new(&mut key_ref, &mut val_ref));
                kv_refs.push(kv_pair);
            }
            let mut kv_pairs: Vec<&mut GCRef> = kv_refs.iter_mut().collect();
            let tuple = gc_system.new_object(VMTuple::new(&mut kv_pairs));
            for mut r in temp_refs {
                r.drop_ref();
            }
            for mut r in kv_refs {
                r.drop_ref();
            }
            Ok(tuple)
        }
    }
}

pub fn json_encode(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_const_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone(),
            format!("json_encode expected 1 argument, got {}", tuple_obj.values.len()),
        ));
    }
    let object_to_encode = tuple_obj.values[0].clone();

    let mut visited = HashSet::default();
    let json_value = vmobject_to_json(object_to_encode, gc_system, &mut visited)?;

    match serde_json::to_string(&json_value) {
        Ok(json_string) => Ok(gc_system.new_object(VMString::new(&json_string))),
        Err(e) => Err(VMVariableError::ValueError(
            tuple.clone(),
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
            format!("json_decode expected 1 argument, got {}", tuple_obj.values.len()),
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
            json_string_obj,
            format!("Failed to parse JSON string: {}", e),
        )),
    }
}

// Helper to provide functions for registration
pub fn get_serialization_functions() -> Vec<(
    &'static str,
    fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("json_encode", json_encode),
        ("json_decode", json_decode),
    ]
}