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
            VMBoolean, VMInt, VMNull, VMString, VMTuple, VMVariableError
        },
        gc::{GCRef, GCSystem},
    };

    use super::{build_module, check_if_tuple}; // Import necessary items

    // Helper to extract a specific string argument from the tuple by index
    fn get_string_arg(args_tuple: &GCRef, index: usize, func_name: &str, arg_name: &str) -> Result<String, VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        if index >= tuple_obj.values.len() {
             return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("{} missing argument: {}", func_name, arg_name),
            ));
        }
        let arg_obj = &tuple_obj.values[index];
        if !arg_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                arg_obj.clone(),
                format!("Argument '{}' for {} must be a string", arg_name, func_name),
            ));
        }
        Ok(arg_obj.as_const_type::<VMString>().value.clone())
    }

     // Helper to extract a specific integer argument from the tuple by index
    fn get_optional_int_arg(args_tuple: &GCRef, index: usize, func_name: &str, arg_name: &str) -> Result<Option<i64>, VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        if index >= tuple_obj.values.len() {
            return Ok(None); // Argument is optional and not provided
        }
        let arg_obj = &tuple_obj.values[index];
         if arg_obj.isinstance::<VMNull>() { // Allow null/None as absence
            return Ok(None);
        }
        if !arg_obj.isinstance::<VMInt>() {
            return Err(VMVariableError::TypeError(
                arg_obj.clone(),
                format!("Argument '{}' for {} must be an integer or null", arg_name, func_name),
            ));
        }
        Ok(Some(arg_obj.as_const_type::<VMInt>().value))
    }

    // Helper to extract a specific tuple argument from the tuple by index
    fn get_tuple_arg(args_tuple: &GCRef, index: usize, func_name: &str, arg_name: &str) -> Result<GCRef, VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
         if index >= tuple_obj.values.len() {
             return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("{} missing argument: {}", func_name, arg_name),
            ));
        }
        let arg_obj = &tuple_obj.values[index];
        if !arg_obj.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                arg_obj.clone(),
                format!("Argument '{}' for {} must be a tuple", arg_name, func_name),
            ));
        }
        Ok(arg_obj.clone())
    }


    // string_utils.split(string, separator, [maxsplit])
    fn split(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if !(2..=3).contains(&arg_count) {
             return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("split expected 2 or 3 arguments, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "split", "string")?;
        let separator = get_string_arg(&args_tuple, 1, "split", "separator")?;

        // Optional maxsplit argument
        let maxsplit_opt_i64 = get_optional_int_arg(&args_tuple, 2, "split", "maxsplit")?;
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
        // ... rest of split implementation remains the same ...
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
    fn join(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if arg_count != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("join expected 2 arguments, got {}", arg_count),
            ));
        }

        let separator = get_string_arg(&args_tuple, 0, "join", "separator")?;
        let iterable_obj = get_tuple_arg(&args_tuple, 1, "join", "iterable")?;
        let iterable_tuple = iterable_obj.as_const_type::<VMTuple>();

        let mut string_parts = Vec::with_capacity(iterable_tuple.values.len());
        for item in &iterable_tuple.values {
            if !item.isinstance::<VMString>() {
                return Err(VMVariableError::TypeError(
                    item.clone(),
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
    fn replace(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if !(3..=4).contains(&arg_count) {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                 format!("replace expected 3 or 4 arguments, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "replace", "string")?;
        let old_str = get_string_arg(&args_tuple, 1, "replace", "old")?;
        let new_str = get_string_arg(&args_tuple, 2, "replace", "new")?;

        // Optional count argument
        let count_opt_i64 = get_optional_int_arg(&args_tuple, 3, "replace", "count")?;
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
    fn startswith(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if arg_count != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("startswith expected 2 arguments, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "startswith", "string")?;
        let prefix_str = get_string_arg(&args_tuple, 1, "startswith", "prefix")?;

        let result = target_str.starts_with(&prefix_str);
        Ok(gc_system.new_object(VMBoolean::new(result)))
    }

    // string_utils.endswith(string, suffix)
    fn endswith(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if arg_count != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("endswith expected 2 arguments, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "endswith", "string")?;
        let suffix_str = get_string_arg(&args_tuple, 1, "endswith", "suffix")?;

        let result = target_str.ends_with(&suffix_str);
        Ok(gc_system.new_object(VMBoolean::new(result)))
    }

    // string_utils.strip(string, [chars])
    fn strip(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if !(1..=2).contains(&arg_count) {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("strip expected 1 or 2 arguments, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "strip", "string")?;

        let result_string = if arg_count > 1 {
             let chars_str = get_string_arg(&args_tuple, 1, "strip", "chars")?;
             let chars_to_strip: Vec<char> = chars_str.chars().collect();
             target_str.trim_matches(|c| chars_to_strip.contains(&c)).to_string()
        } else {
            // Default: strip whitespace
            target_str.trim().to_string()
        };
        // ... rest of strip implementation remains the same ...
        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // string_utils.lower(string)
    fn lower(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if arg_count != 1 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("lower expected 1 argument, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "lower", "string")?;
        let result_string = target_str.to_lowercase();
        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // string_utils.upper(string)
    fn upper(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        let arg_count = tuple_obj.values.len();

        if arg_count != 1 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("upper expected 1 argument, got {}", arg_count),
            ));
        }

        let target_str = get_string_arg(&args_tuple, 0, "upper", "string")?;
        let result_string = target_str.to_uppercase();
        Ok(gc_system.new_object(VMString::new(&result_string)))
    }

    // Function to create the string utility module
    pub fn get_string_utils_module(gc_system: &mut GCSystem) -> GCRef {
        let mut functions = FxHashMap::default();
        // ... existing function insertions ...
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

mod fs {
    use rustc_hash::FxHashMap;
    use std::{
        fs::{self, OpenOptions},
        io::Write,
        path::Path,
    };

    use crate::{
        executor::variable::{
            VMBoolean, VMBytes, VMNull, VMString, VMTuple, VMVariableError,
        },
        gc::{GCRef, GCSystem},
    };

    use super::{build_module, check_if_tuple}; // Import necessary items

    // Helper to map std::io::Error to VMVariableError
    fn io_error_to_vm(err: std::io::Error, path: Option<&str>) -> VMVariableError {
        let msg = match path {
            Some(p) => format!("IO Error for path '{}': {}", p, err),
            None => format!("IO Error: {}", err),
        };
        VMVariableError::DetailedError(msg)
    }

    // Helper to extract a single string argument (path)
    fn get_path_arg(args_tuple: &GCRef, func_name: &str) -> Result<String, VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("{} requires exactly one argument (path)", func_name),
            ));
        }
        let path_obj = &tuple_obj.values[0];
        if !path_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                path_obj.clone(),
                "Path argument must be a string".to_string(),
            ));
        }
        Ok(path_obj.as_const_type::<VMString>().value.clone())
    }

    // Helper to extract path and content (string) arguments
    fn get_path_content_string_args(
        args_tuple: &GCRef,
        func_name: &str,
    ) -> Result<(String, String), VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("{} requires exactly two arguments (path, content)", func_name),
            ));
        }
        let path_obj = &tuple_obj.values[0];
        let content_obj = &tuple_obj.values[1];

        if !path_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                path_obj.clone(),
                "Path argument must be a string".to_string(),
            ));
        }
         if !content_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                content_obj.clone(),
                "Content argument must be a string".to_string(),
            ));
        }
        let path = path_obj.as_const_type::<VMString>().value.clone();
        let content = content_obj.as_const_type::<VMString>().value.clone();
        Ok((path, content))
    }

     // Helper to extract path and content (bytes) arguments
    fn get_path_content_bytes_args(
        args_tuple: &GCRef,
        func_name: &str,
    ) -> Result<(String, Vec<u8>), VMVariableError> {
        let tuple_obj = args_tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 2 {
            return Err(VMVariableError::TypeError(
                args_tuple.clone(),
                format!("{} requires exactly two arguments (path, content)", func_name),
            ));
        }
        let path_obj = &tuple_obj.values[0];
        let content_obj = &tuple_obj.values[1];

        if !path_obj.isinstance::<VMString>() {
            return Err(VMVariableError::TypeError(
                path_obj.clone(),
                "Path argument must be a string".to_string(),
            ));
        }
         if !content_obj.isinstance::<VMBytes>() {
            return Err(VMVariableError::TypeError(
                content_obj.clone(),
                "Content argument must be bytes".to_string(),
            ));
        }
        let path = path_obj.as_const_type::<VMString>().value.clone();
        let content = content_obj.as_const_type::<VMBytes>().value.clone();
        Ok((path, content))
    }


    // fs.read(path) -> string
    fn read_file(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "read")?;
        let path = Path::new(&path_str);

        match fs::read_to_string(path) {
            Ok(content) => Ok(gc_system.new_object(VMString::new(&content))),
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.read_bytes(path) -> bytes
    fn read_bytes(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "read_bytes")?;
        let path = Path::new(&path_str);

        match fs::read(path) {
            Ok(content) => Ok(gc_system.new_object(VMBytes::new(&content))),
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.write(path, content)
    fn write_file(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let (path_str, content) = get_path_content_string_args(&args_tuple, "write")?;
        let path = Path::new(&path_str);

        match fs::write(path, content) {
            Ok(_) => Ok(gc_system.new_object(VMNull::new())),
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.write_bytes(path, content)
    fn write_bytes(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let (path_str, content) = get_path_content_bytes_args(&args_tuple, "write_bytes")?;
        let path = Path::new(&path_str);

        match fs::write(path, content) {
            Ok(_) => Ok(gc_system.new_object(VMNull::new())),
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

     // fs.append(path, content)
    fn append_file(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let (path_str, content) = get_path_content_string_args(&args_tuple, "append")?;
        let path = Path::new(&path_str);

        match OpenOptions::new().append(true).create(true).open(path) {
            Ok(mut file) => match file.write_all(content.as_bytes()) {
                Ok(_) => Ok(gc_system.new_object(VMNull::new())),
                Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
            },
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.append_bytes(path, content)
    fn append_bytes(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let (path_str, content) = get_path_content_bytes_args(&args_tuple, "append_bytes")?;
        let path = Path::new(&path_str);

         match OpenOptions::new().append(true).create(true).open(path) {
            Ok(mut file) => match file.write_all(&content) {
                Ok(_) => Ok(gc_system.new_object(VMNull::new())),
                Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
            },
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }


    // fs.exists(path) -> bool
    fn exists(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "exists")?;
        let path = Path::new(&path_str);
        Ok(gc_system.new_object(VMBoolean::new(path.exists())))
    }

    // fs.is_file(path) -> bool
    fn is_file(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "is_file")?;
        let path = Path::new(&path_str);
        Ok(gc_system.new_object(VMBoolean::new(path.is_file())))
    }

    // fs.is_dir(path) -> bool
    fn is_dir(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "is_dir")?;
        let path = Path::new(&path_str);
        Ok(gc_system.new_object(VMBoolean::new(path.is_dir())))
    }

    // fs.remove(path)
    fn remove(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "remove")?;
        let path = Path::new(&path_str);

        let result = if path.is_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        };

        match result {
            Ok(_) => Ok(gc_system.new_object(VMNull::new())),
            // Return false if path does not exist, mimic os.remove behavior?
            // Or stick to raising error? Let's raise error for now.
             Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                 Err(VMVariableError::ValueError(args_tuple.clone(), format!("Path not found: {}", path_str)))
            }
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.mkdir(path)
    fn mkdir(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "mkdir")?;
        let path = Path::new(&path_str);

        match fs::create_dir_all(path) { // create_dir_all is like mkdir -p
            Ok(_) => Ok(gc_system.new_object(VMNull::new())),
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }

    // fs.listdir(path) -> tuple<string>
    fn listdir(args_tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(args_tuple.clone())?;
        let path_str = get_path_arg(&args_tuple, "listdir")?;
        let path = Path::new(&path_str);

        match fs::read_dir(path) {
            Ok(entries) => {
                let mut result_elements = Vec::new();

                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            // Get filename as string, handle potential non-UTF8 names
                            let file_name = entry.file_name();
                            match file_name.to_str() {
                                Some(name_str) => {
                                    let vm_name = gc_system.new_object(VMString::new(name_str));
                                    result_elements.push(vm_name.clone());
                                }
                                None => {
                                     // Handle non-UTF8 filenames if necessary, e.g., skip or error
                                     // For simplicity, we skip here.
                                     eprintln!("Warning: Skipping non-UTF8 filename in listdir");
                                }
                            }
                        }
                        Err(e) => {
                            return Err(io_error_to_vm(e, Some(&path_str)));
                        }
                    }
                }

                let mut element_refs: Vec<&mut GCRef> = result_elements.iter_mut().collect();
                let result_tuple = gc_system.new_object(VMTuple::new(&mut element_refs));

                // Drop refs owned by the new tuple and temporary refs
                for mut r in result_elements { r.drop_ref(); }

                Ok(result_tuple)
            }
            Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
        }
    }


    // Function to create the fs module
    pub fn get_fs_module(gc_system: &mut GCSystem) -> GCRef {
        let mut functions = FxHashMap::default();
        // Add functions to the map
        functions.insert("read".to_string(), read_file as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("read_bytes".to_string(), read_bytes as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("write".to_string(), write_file as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("write_bytes".to_string(), write_bytes as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("append".to_string(), append_file as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("append_bytes".to_string(), append_bytes as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("exists".to_string(), exists as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("is_file".to_string(), is_file as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("is_dir".to_string(), is_dir as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("remove".to_string(), remove as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("mkdir".to_string(), mkdir as for<'a> fn(_, &'a mut _) -> _);
        functions.insert("listdir".to_string(), listdir as for<'a> fn(_, &'a mut _) -> _);

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
        let mut string_module = super::string_utils::get_string_utils_module(gc_system);
        let result = context.let_var("string_utils", &mut string_module, gc_system);
        string_module.drop_ref();

        if let Err(context_error) = result {
            return Err(VMError::ContextError(context_error));
        }

        // Inject the fs module
        let mut fs_module = super::fs::get_fs_module(gc_system);
        let result = context.let_var("fs", &mut fs_module, gc_system);
        fs_module.drop_ref(); // Drop ref after use

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
                format!("json_encode expected 1 argument, got {}", tuple_obj.values.len()),
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
                format!("len expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
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
                target_obj.clone(), // Error points to the specific object
                "Argument for len must be a string, bytes, or tuple".to_string(),
            ));
        }
    }

    pub fn to_int(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("int expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
        if target_obj.isinstance::<VMInt>() {
            let data = target_obj.as_const_type::<VMInt>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if target_obj.isinstance::<VMFloat>() {
            let data = target_obj.as_const_type::<VMFloat>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if target_obj.isinstance::<VMString>() {
            let data = target_obj.as_const_type::<VMString>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if target_obj.isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMInt::new(0)));
        }
        if target_obj.isinstance::<VMBoolean>() {
            let data = target_obj.as_const_type::<VMBoolean>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        Err(VMVariableError::TypeError(
            target_obj.clone(), // Error points to the specific object
            "Argument for int must be convertible to an integer".to_string(),
        ))
    }

    pub fn to_float(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("float expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
        if target_obj.isinstance::<VMInt>() {
            let data = target_obj.as_const_type::<VMInt>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if target_obj.isinstance::<VMFloat>() {
            let data = target_obj.as_const_type::<VMFloat>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if target_obj.isinstance::<VMString>() {
            let data = target_obj.as_const_type::<VMString>().to_float()?;
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
            target_obj.clone(), // Error points to the specific object
            "Argument for float must be convertible to a float".to_string(),
        ))
    }

    pub fn to_string(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("string expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
        // Use the existing try_to_string_vmobject helper which handles various types
        let data = try_to_string_vmobject(target_obj.clone(), None)?;
        Ok(gc_system.new_object(VMString::new(&data)))
    }

    pub fn to_bool(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("bool expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
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
            target_obj.clone(), // Error points to the specific object
            "Argument for bool must be convertible to a boolean".to_string(),
        ))
    }
    pub fn to_bytes(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("bytes expected 1 argument, got {}", tuple_obj.values.len()),
            ));
        }
        let target_obj = &tuple_obj.values[0];
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
                    target_obj.clone(),
                    "Integer values for bytes conversion must be between 0 and 255".to_string(),
                ));
            }
            return Ok(gc_system.new_object(VMBytes::new(&vec![int_value as u8])));
        } else if target_obj.isinstance::<VMTuple>() {
            // 支持整数元组转字节序列
            let inner_tuple = target_obj.as_const_type::<VMTuple>();
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
            target_obj.clone(), // Error points to the specific object
            "Argument for bytes must be bytes, string, integer (0-255), or tuple of integers (0-255)"
                .to_string(),
        ))
    }

    pub fn input(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() > 1 {
             return Err(VMVariableError::TypeError(
                tuple.clone(),
                format!("input expected 0 or 1 arguments, got {}", tuple_obj.values.len()),
            ));
        }

        let prompt = if tuple_obj.values.len() == 1 {
            let prompt_obj = &tuple_obj.values[0];
             if !prompt_obj.isinstance::<VMString>() {
                 return Err(VMVariableError::TypeError(
                    prompt_obj.clone(),
                    "Argument to input (prompt) must be a string".to_string(),
                ));
            }
            prompt_obj.as_const_type::<VMString>().value.clone()
        } else {
            "".to_string() // Default empty prompt
        };

        print!("{}", prompt); // Use {} instead of "{ } "
        std::io::stdout().flush().unwrap_or(());
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let data = input.trim_end_matches(|c| c == '\r' || c == '\n').to_string(); // Trim newline/CRLF
        return Ok(gc_system.new_object(VMString::new(&data)));
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
}
