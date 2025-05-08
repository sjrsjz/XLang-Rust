use xlang_vm_core::{
    executor::variable::{VMBoolean, VMInt, VMNull, VMString, VMTuple, VMVariableError},
    gc::{GCRef, GCSystem},
};

use super::check_if_tuple; // Import necessary items

// Helper to extract a specific string argument from the tuple by index
fn get_string_arg(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    index: usize,
    func_name: &str,
    arg_name: &str,
) -> Result<String, VMVariableError> {
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    if index >= tuple_obj.values.len() {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("{} missing argument: {}", func_name, arg_name),
        ));
    }
    let arg_obj = &mut tuple_obj.values[index];
    if !arg_obj.isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            arg_obj.clone_ref(),
            format!("Argument '{}' for {} must be a string", arg_name, func_name),
        ));
    }
    Ok(arg_obj.as_const_type::<VMString>().value.clone())
}

// Helper to extract a specific integer argument from the tuple by index
fn get_optional_int_arg(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    index: usize,
    func_name: &str,
    arg_name: &str,
) -> Result<Option<i64>, VMVariableError> {
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    if index >= tuple_obj.values.len() {
        return Ok(None); // Argument is optional and not provided
    }
    let arg_obj = &mut tuple_obj.values[index];
    if arg_obj.isinstance::<VMNull>() {
        // Allow null/None as absence
        return Ok(None);
    }
    if !arg_obj.isinstance::<VMInt>() {
        return Err(VMVariableError::TypeError(
            arg_obj.clone_ref(),
            format!(
                "Argument '{}' for {} must be an integer or null",
                arg_name, func_name
            ),
        ));
    }
    Ok(Some(arg_obj.as_const_type::<VMInt>().value))
}

// Helper to extract a specific tuple argument from the tuple by index
fn get_tuple_arg<'t>(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &'t mut GCRef,
    index: usize,
    func_name: &str,
    arg_name: &str,
) -> Result<&'t mut GCRef, VMVariableError> {
    if index >= args_tuple.as_const_type::<VMTuple>().values.len() {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("{} missing argument: {}", func_name, arg_name),
        ));
    }
    let arg_obj = &mut args_tuple.as_type::<VMTuple>().values[index];
    if !arg_obj.isinstance::<VMTuple>() {
        return Err(VMVariableError::TypeError(
            arg_obj.clone_ref(),
            format!("Argument '{}' for {} must be a tuple", arg_name, func_name),
        ));
    }
    Ok(arg_obj)
}

// string_utils.split(string, separator, [maxsplit])
fn split(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if !(2..=3).contains(&arg_count) {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("split expected 2 or 3 arguments, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "split", "string")?;
    let separator = get_string_arg(None, None, args_tuple, 1, "split", "separator")?;

    // Optional maxsplit argument
    let maxsplit_opt_i64 = get_optional_int_arg(None, None, args_tuple, 2, "split", "maxsplit")?;
    let maxsplit: Option<usize> = match maxsplit_opt_i64 {
        Some(val) if val < 0 => None, // Negative maxsplit means split all
        Some(val) => Some(val as usize),
        None => None, // Default: split all occurrences or not provided
    };

    let mut result_elements = Vec::new();

    let parts: Vec<&str> = match maxsplit {
        Some(n) => target_str.splitn(n + 1, &separator).collect(),
        None => target_str.split(&separator).collect(),
    };
    for part in parts {
        let vm_part = gc_system.new_object(VMString::new(part));
        result_elements.push(vm_part.clone()); // Clone for the final tuple
    }

    let mut element_refs: Vec<&mut GCRef> = result_elements.iter_mut().collect();
    let result_tuple = gc_system.new_object(VMTuple::new(&mut element_refs));

    for mut r in result_elements {
        r.drop_ref();
    }

    Ok(result_tuple)
}

// string_utils.join(separator, iterable)
fn join(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if arg_count != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("join expected 2 arguments, got {}", arg_count),
        ));
    }

    let separator = get_string_arg(None, None, args_tuple, 0, "join", "separator")?;
    let iterable_obj = get_tuple_arg(None, None, args_tuple, 1, "join", "iterable")?;
    let iterable_tuple = iterable_obj.as_type::<VMTuple>();

    let mut string_parts = Vec::with_capacity(iterable_tuple.values.len());
    for item in &mut iterable_tuple.values {
        if !item.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                item.clone_ref(),
                "All elements in the iterable for join must be strings".to_string(),
            ));
        }
        string_parts.push(item.as_const_type::<VMString>().value.as_str());
    }
    // ... rest of join implementation remains the same ...
    let joined_string = string_parts.join(&separator);
    Ok(gc_system.new_object(VMString::new(&joined_string)))
}

// string_utils.replace(string, old, new, [count])
fn replace(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if !(3..=4).contains(&arg_count) {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("replace expected 3 or 4 arguments, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "replace", "string")?;
    let old_str = get_string_arg(None, None, args_tuple, 1, "replace", "old")?;
    let new_str = get_string_arg(None, None, args_tuple, 2, "replace", "new")?;

    // Optional count argument
    let count_opt_i64 = get_optional_int_arg(None, None, args_tuple, 3, "replace", "count")?;
    let count: Option<usize> = match count_opt_i64 {
        Some(val) if val < 0 => None, // Negative count means replace all
        Some(val) => Some(val as usize),
        None => None, // Default: replace all occurrences or not provided
    };

    let result_string = match count {
        Some(n) => target_str.replacen(&old_str, &new_str, n),
        None => target_str.replace(&old_str, &new_str),
    };
    // ... rest of replace implementation remains the same ...
    Ok(gc_system.new_object(VMString::new(&result_string)))
}

// string_utils.startswith(string, prefix)
fn startswith(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if arg_count != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("startswith expected 2 arguments, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "startswith", "string")?;
    let prefix_str = get_string_arg(None, None, args_tuple, 1, "startswith", "prefix")?;

    let result = target_str.starts_with(&prefix_str);
    Ok(gc_system.new_object(VMBoolean::new(result)))
}

// string_utils.endswith(string, suffix)
fn endswith(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if arg_count != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("endswith expected 2 arguments, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "endswith", "string")?;
    let suffix_str = get_string_arg(None, None, args_tuple, 1, "endswith", "suffix")?;

    let result = target_str.ends_with(&suffix_str);
    Ok(gc_system.new_object(VMBoolean::new(result)))
}

// string_utils.strip(string, [chars])
fn strip(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if !(1..=2).contains(&arg_count) {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("strip expected 1 or 2 arguments, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "strip", "string")?;

    let result_string = if arg_count > 1 {
        let chars_str = get_string_arg(None, None, args_tuple, 1, "strip", "chars")?;
        let chars_to_strip: Vec<char> = chars_str.chars().collect();
        target_str
            .trim_matches(|c| chars_to_strip.contains(&c))
            .to_string()
    } else {
        // Default: strip whitespace
        target_str.trim().to_string()
    };
    // ... rest of strip implementation remains the same ...
    Ok(gc_system.new_object(VMString::new(&result_string)))
}

// string_utils.lower(string)
fn lower(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if arg_count != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("lower expected 1 argument, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "lower", "string")?;
    let result_string = target_str.to_lowercase();
    Ok(gc_system.new_object(VMString::new(&result_string)))
}

// string_utils.upper(string)
fn upper(
    _self_object: Option<&mut GCRef>,
    _capture: Option<&mut GCRef>,
    args_tuple: &mut GCRef,
    gc_system: &mut GCSystem,
) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let tuple_obj = args_tuple.as_const_type::<VMTuple>();
    let arg_count = tuple_obj.values.len();

    if arg_count != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("upper expected 1 argument, got {}", arg_count),
        ));
    }

    let target_str = get_string_arg(None, None, args_tuple, 0, "upper", "string")?;
    let result_string = target_str.to_uppercase();
    Ok(gc_system.new_object(VMString::new(&result_string)))
}

// Helper to provide functions for registration
pub fn get_string_utils_module() -> Vec<(
    &'static str,
    fn(
        Option<&mut GCRef>,
        Option<&mut GCRef>,
        &mut GCRef,
        &mut GCSystem,
    ) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("split", split),
        ("join", join),
        ("replace", replace),
        ("startswith", startswith),
        ("endswith", endswith),
        ("strip", strip),
        ("lower", lower),
        ("upper", upper),
    ]
}
