use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use xlang_vm_core::{
    executor::variable::{VMBoolean, VMBytes, VMNull, VMString, VMTuple, VMVariableError},
    gc::{GCRef, GCSystem},
};

use super::check_if_tuple; // Import necessary items

// Helper to map std::io::Error to VMVariableError
fn io_error_to_vm(err: std::io::Error, path: Option<&str>) -> VMVariableError {
    let msg = match path {
        Some(p) => format!("IO Error for path '{}': {}", p, err),
        None => format!("IO Error: {}", err),
    };
    VMVariableError::DetailedError(msg)
}

// Helper to extract a single string argument (path)
fn get_path_arg(args_tuple: &mut GCRef, func_name: &str) -> Result<String, VMVariableError> {
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 1 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!("{} requires exactly one argument (path)", func_name),
        ));
    }
    let path_obj = &mut tuple_obj.values[0];
    if !path_obj.isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            path_obj.clone_ref(),
            "Path argument must be a string".to_string(),
        ));
    }
    Ok(path_obj.as_const_type::<VMString>().value.clone())
}

// Helper to extract path and content (string) arguments
fn get_path_content_string_args(
    args_tuple: &mut GCRef,
    func_name: &str,
) -> Result<(String, String), VMVariableError> {
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!(
                "{} requires exactly two arguments (path, content)",
                func_name
            ),
        ));
    }
    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "Path argument must be a string".to_string(),
        ));
    }
    if !tuple_obj.values[1].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[1].clone_ref(),
            "Content argument must be a string".to_string(),
        ));
    }
    let path = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let content = tuple_obj.values[1].as_const_type::<VMString>().value.clone();
    Ok((path, content))
}

// Helper to extract path and content (bytes) arguments
fn get_path_content_bytes_args(
    args_tuple: &mut GCRef,
    func_name: &str,
) -> Result<(String, Vec<u8>), VMVariableError> {
    let tuple_obj = args_tuple.as_type::<VMTuple>();
    if tuple_obj.values.len() != 2 {
        return Err(VMVariableError::TypeError(
            args_tuple.clone_ref(),
            format!(
                "{} requires exactly two arguments (path, content)",
                func_name
            ),
        ));
    }

    if !tuple_obj.values[0].isinstance::<VMString>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[0].clone_ref(),
            "Path argument must be a string".to_string(),
        ));
    }
    if !tuple_obj.values[1].isinstance::<VMBytes>() {
        return Err(VMVariableError::TypeError(
            tuple_obj.values[1].clone_ref(),
            "Content argument must be bytes".to_string(),
        ));
    }
    let path = tuple_obj.values[0].as_const_type::<VMString>().value.clone();
    let content = tuple_obj.values[1].as_const_type::<VMBytes>().value.clone();
    Ok((path, content))
}

// fs.read(path) -> string
fn read_file(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "read")?;
    let path = Path::new(&path_str);

    match fs::read_to_string(path) {
        Ok(content) => Ok(gc_system.new_object(VMString::new(&content))),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.read_bytes(path) -> bytes
fn read_bytes(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "read_bytes")?;
    let path = Path::new(&path_str);

    match fs::read(path) {
        Ok(content) => Ok(gc_system.new_object(VMBytes::new(&content))),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.write(path, content)
fn write_file(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let (path_str, content) = get_path_content_string_args(args_tuple, "write")?;
    let path = Path::new(&path_str);

    match fs::write(path, content) {
        Ok(_) => Ok(gc_system.new_object(VMNull::new())),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.write_bytes(path, content)
fn write_bytes(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let (path_str, content) = get_path_content_bytes_args(args_tuple, "write_bytes")?;
    let path = Path::new(&path_str);

    match fs::write(path, content) {
        Ok(_) => Ok(gc_system.new_object(VMNull::new())),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.append(path, content)
fn append_file(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let (path_str, content) = get_path_content_string_args(args_tuple, "append")?;
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
fn append_bytes(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let (path_str, content) = get_path_content_bytes_args(args_tuple, "append_bytes")?;
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
fn exists(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "exists")?;
    let path = Path::new(&path_str);
    Ok(gc_system.new_object(VMBoolean::new(path.exists())))
}

// fs.is_file(path) -> bool
fn is_file(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "is_file")?;
    let path = Path::new(&path_str);
    Ok(gc_system.new_object(VMBoolean::new(path.is_file())))
}

// fs.is_dir(path) -> bool
fn is_dir(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "is_dir")?;
    let path = Path::new(&path_str);
    Ok(gc_system.new_object(VMBoolean::new(path.is_dir())))
}

// fs.remove(path)
fn remove(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "remove")?;
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(VMVariableError::ValueError(
            args_tuple.clone(),
            format!("Path not found: {}", path_str),
        )),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.mkdir(path)
fn mkdir(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "mkdir")?;
    let path = Path::new(&path_str);

    match fs::create_dir_all(path) {
        // create_dir_all is like mkdir -p
        Ok(_) => Ok(gc_system.new_object(VMNull::new())),
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// fs.listdir(path) -> tuple<string>
fn listdir(args_tuple: &mut GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(args_tuple)?;
    let path_str = get_path_arg(args_tuple, "listdir")?;
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
            for mut r in result_elements {
                r.drop_ref();
            }

            Ok(result_tuple)
        }
        Err(e) => Err(io_error_to_vm(e, Some(&path_str))),
    }
}

// Helper to provide functions for registration
pub fn get_fs_module() -> Vec<(
    &'static str,
    fn(&mut GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("read", read_file),
        ("read_bytes", read_bytes),
        ("write", write_file),
        ("write_bytes", write_bytes),
        ("append", append_file),
        ("append_bytes", append_bytes),
        ("exists", exists),
        ("is_file", is_file),
        ("is_dir", is_dir),
        ("remove", remove),
        ("mkdir", mkdir),
        ("listdir", listdir),
    ]
}