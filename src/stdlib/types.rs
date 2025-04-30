use xlang_vm_core::{
    executor::variable::{
        try_to_string_vmobject,
        // Import necessary VM types
        VMBoolean, VMBytes, VMFloat, VMInt, VMNull, VMString, VMTuple, VMVariableError,
    },
    gc::{GCRef, GCSystem},
};
// Assuming check_if_tuple will be available via super
use super::check_if_tuple;

pub fn len(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("len expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMTuple>() {
        let inner_tuple = target_obj.as_const_type::<VMTuple>();
        let obj = gc_system.new_object(VMInt::new(inner_tuple.values.len() as i64));
        Ok(obj)
    } else if target_obj.isinstance::<VMString>() {
        let inner_string = target_obj.as_const_type::<VMString>();
        let obj = gc_system.new_object(VMInt::new(inner_string.value.len() as i64));
        return Ok(obj);
    } else if target_obj.isinstance::<VMBytes>() {
        let inner_bytes = target_obj.as_const_type::<VMBytes>();
        let obj = gc_system.new_object(VMInt::new(inner_bytes.value.len() as i64));
        return Ok(obj);
    } else {
        return Err(VMVariableError::TypeError(
            target_obj.clone_ref(), // Error points to the specific object
            "Argument for len must be a string, bytes, or tuple".to_string(),
        ));
    }
}

pub fn to_int(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("int expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMInt>() {
        let data = target_obj.as_type::<VMInt>().to_int()?;
        return Ok(gc_system.new_object(VMInt::new(data)));
    }
    if target_obj.isinstance::<VMFloat>() {
        let data = target_obj.as_type::<VMFloat>().to_int()?;
        return Ok(gc_system.new_object(VMInt::new(data)));
    }
    if target_obj.isinstance::<VMString>() {
        let data = target_obj.as_type::<VMString>().to_int()?;
        return Ok(gc_system.new_object(VMInt::new(data)));
    }
    if target_obj.isinstance::<VMNull>() {
        return Ok(gc_system.new_object(VMInt::new(0)));
    }
    if target_obj.isinstance::<VMBoolean>() {
        let data = target_obj.as_type::<VMBoolean>().to_int()?;
        return Ok(gc_system.new_object(VMInt::new(data)));
    }
    Err(VMVariableError::TypeError(
        target_obj.clone_ref(), // Error points to the specific object
        "Argument for int must be convertible to an integer".to_string(),
    ))
}

pub fn to_float(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("float expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMInt>() {
        let data = target_obj.as_type::<VMInt>().to_float()?;
        return Ok(gc_system.new_object(VMFloat::new(data)));
    }
    if target_obj.isinstance::<VMFloat>() {
        let data = target_obj.as_type::<VMFloat>().to_float()?;
        return Ok(gc_system.new_object(VMFloat::new(data)));
    }
    if target_obj.isinstance::<VMString>() {
        let data = target_obj.as_type::<VMString>().to_float()?;
        return Ok(gc_system.new_object(VMFloat::new(data)));
    }
    if target_obj.isinstance::<VMNull>() {
        return Ok(gc_system.new_object(VMFloat::new(0.0)));
    }
    if target_obj.isinstance::<VMBoolean>() {
        let data = target_obj
            .as_const_type::<VMBoolean>()
            .to_float()?;
        return Ok(gc_system.new_object(VMFloat::new(data)));
    }
    Err(VMVariableError::TypeError(
        target_obj.clone_ref(), // Error points to the specific object
        "Argument for float must be convertible to a float".to_string(),
    ))
}

pub fn to_string(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("string expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMBytes>() {
        let data = target_obj.as_type::<VMBytes>().to_string()?;
        return Ok(gc_system.new_object(VMString::new(&data)));
    }
    // Use the try_to_string_vmobject helper defined/re-exported in this module
    let data = try_to_string_vmobject(target_obj, None)?;
    Ok(gc_system.new_object(VMString::new(&data)))
}

pub fn to_bool(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("bool expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMInt>() {
        let data = target_obj.as_const_type::<VMInt>().to_bool()?;
        return Ok(gc_system.new_object(VMBoolean::new(data)));
    }
    if target_obj.isinstance::<VMFloat>() {
        let data = target_obj.as_const_type::<VMFloat>().to_bool()?;
        return Ok(gc_system.new_object(VMBoolean::new(data)));
    }
    if target_obj.isinstance::<VMString>() {
        let data = target_obj.as_const_type::<VMString>().to_bool()?;
        return Ok(gc_system.new_object(VMBoolean::new(data)));
    }
    if target_obj.isinstance::<VMNull>() {
        return Ok(gc_system.new_object(VMBoolean::new(false)));
    }
    if target_obj.isinstance::<VMBoolean>() {
        let data = target_obj.as_const_type::<VMBoolean>().to_bool()?;
        return Ok(gc_system.new_object(VMBoolean::new(data)));
    }
     Err(VMVariableError::TypeError(
        target_obj.clone_ref(), // Error points to the specific object
        "Argument for bool must be convertible to a boolean".to_string(),
    ))
}
pub fn to_bytes(tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple)?;
    let tuple_obj = tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            tuple.clone_ref(),
            format!("bytes expected 1 argument, got {}", tuple.as_const_type::<VMTuple>().values.len()),
        ));
    }
    let target_obj = &mut tuple_obj.values[0];
    if target_obj.isinstance::<VMBytes>() {
        return Ok(gc_system.new_object(VMBytes::new(
            &target_obj.as_const_type::<VMBytes>().value,
        )));
    } else if target_obj.isinstance::<VMString>() {
        // 将字符串转换为字节序列
        let string_value = target_obj
            .as_const_type::<VMString>()
            .value
            .clone();
        return Ok(gc_system.new_object(VMBytes::new(&string_value.as_bytes().to_vec())));
    } else if target_obj.isinstance::<VMInt>() {
        // 支持单字节的整数转字节
        let int_value = target_obj.as_const_type::<VMInt>().value;
        if !(0..=255).contains(&int_value) {
            return Err(VMVariableError::ValueError(
                target_obj.clone_ref(),
                "Integer values for bytes conversion must be between 0 and 255".to_string(),
            ));
        }
        return Ok(gc_system.new_object(VMBytes::new(&vec![int_value as u8])));
    } else if target_obj.isinstance::<VMTuple>() {
        // 支持整数元组转字节序列
        let inner_tuple = target_obj.as_type::<VMTuple>();
        let mut byte_vec = Vec::with_capacity(inner_tuple.values.len());

        for value in &mut inner_tuple.values {
            if !value.isinstance::<VMInt>() {
                return Err(VMVariableError::ValueError(
                    value.clone_ref(),
                    "All elements in tuple must be integers for bytes conversion".to_string(),
                ));
            }

            let int_value = value.as_const_type::<VMInt>().value;
            if !(0..=255).contains(&int_value) {
                return Err(VMVariableError::ValueError(
                    value.clone_ref(),
                    "Integer values for bytes conversion must be between 0 and 255".to_string(),
                ));
            }

            byte_vec.push(int_value as u8);
        }

        return Ok(gc_system.new_object(VMBytes::new(&byte_vec)));
    }

    Err(VMVariableError::TypeError(
        target_obj.clone_ref(), // Error points to the specific object
        "Argument for bytes must be bytes, string, integer (0-255), or tuple of integers (0-255)"
            .to_string(),
    ))
}

// Helper to provide functions for registration
pub fn get_type_conversion_functions() -> Vec<(
    &'static str,
    fn(&mut GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("len", len),
        ("int", to_int),
        ("float", to_float),
        ("string", to_string),
        ("bool", to_bool),
        ("bytes", to_bytes),
    ]
}