use std::io::Write;
use xlang_vm_core::{
    executor::variable::{try_to_string_vmobject, VMNull, VMString, VMTuple, VMVariableError},
    gc::{GCRef, GCSystem},
};
use super::check_if_tuple;

pub fn print(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
    check_if_tuple(tuple.clone())?;
    let tuple_obj = tuple.as_const_type::<VMTuple>(); // Renamed from 'tuple' to avoid shadowing
    let mut result = String::new();
    for obj in &tuple_obj.values {
        // Use the imported try_to_string_vmobject
        let repr = try_to_string_vmobject(obj.clone(), None)?;
        result.push_str(&format!("{} ", repr));
    }
    result = result.trim_end_matches(" ").to_string();
    println!("{}", result);
    let obj = gc_system.new_object(VMNull::new());
    Ok(obj)
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

// Helper to provide functions for registration
pub fn get_io_functions() -> Vec<(
    &'static str,
    fn(GCRef, &mut GCSystem) -> Result<GCRef, VMVariableError>,
)> {
    vec![
        ("print", print),
        ("input", input),
    ]
}