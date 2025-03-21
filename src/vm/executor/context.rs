use std::collections::HashMap;

use super::super::gc::gc::GCRef;
use super::super::gc::gc::GCSystem;
use super::variable::try_assign_as_vmobject;
use super::variable::try_repr_vmobject;
use super::variable::VMStackObject;
use super::variable::VMVariableError;
use super::variable::VMVariableWrapper;

#[derive(Debug)]
pub struct Context {
    pub(crate) frames: Vec<(HashMap<String, GCRef>, bool, usize, bool)>, // vars, is_function_frame, function_code_position, is_hidden_frame
    pub(crate) stack_pointers: Vec<usize>,
}

#[derive(Debug)]
pub enum ContextError {
    NoFrame,
    NoVariable(String),
    ExistingVariable(String),
    OfflinedObject(GCRef),
    InvalidContextVariable(GCRef),
    VMVariableError(VMVariableError),
    ContextError(String),
}

impl ContextError {
    pub fn to_string(&self) -> String {
        match self {
            ContextError::NoFrame => "No frame".to_string(),
            ContextError::NoVariable(name) => format!("No variable: {}", name),
            ContextError::ExistingVariable(name) => format!("Existing variable: {}", name),
            ContextError::OfflinedObject(obj) => format!(
                "Offlined object: {:?}",
                try_repr_vmobject(obj.clone(), None).unwrap_or(format!("{:?}", obj))
            ),
            ContextError::InvalidContextVariable(obj) => format!(
                "Invalid context variable: {:?}",
                try_repr_vmobject(obj.clone(), None).unwrap_or(format!("{:?}", obj))
            ),
            ContextError::VMVariableError(err) => {
                format!("VM variable error: {:?}", err.to_string())
            }
            ContextError::ContextError(msg) => format!("Context error: {}", msg),
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            frames: Vec::new(),
            stack_pointers: Vec::new(),
        }
    }

    fn is_variable(&self, object: &GCRef) -> bool {
        if object.isinstance::<VMVariableWrapper>() {
            return true;
        }
        return false;
    }

    fn offline_if_not_variable(&self, object: &GCRef) {
        if !self.is_variable(object) {
            object.offline();
        }
    }

    pub fn new_frame(
        &mut self,
        stack: &Vec<VMStackObject>,
        is_function_frame: bool,
        function_code_position: usize,
        is_hidden_frame: bool,
    ) {
        self.frames.push((
            HashMap::new(),
            is_function_frame,
            function_code_position,
            is_hidden_frame,
        ));
        self.stack_pointers.push(stack.len());
    }

    pub fn pop_frame(
        &mut self,
        stack: &mut Vec<VMStackObject>,
        exit_function: bool,
    ) -> Result<(), ContextError> {
        if exit_function {
            while self.frames.len() > 0 && !self.frames[self.frames.len() - 1].1 {
                for variable in self.frames.last_mut().unwrap().0.values() {
                    variable.offline();
                }
                self.frames.pop();
                let pointer = self.stack_pointers.pop();

                for i in pointer.unwrap_or(0)..stack.len() {
                    if let VMStackObject::VMObject(obj_ref) = &mut stack[i] {
                        self.offline_if_not_variable(obj_ref);
                    }
                }

                stack.truncate(pointer.unwrap_or(0));
            }
        }
        if self.frames.is_empty() {
            return Err(ContextError::NoFrame);
        }
        for variable in self.frames.last_mut().unwrap().0.values() {
            variable.offline();
        }
        self.frames.pop();
        let pointer = self.stack_pointers.pop();
        for i in pointer.unwrap_or(0)..stack.len() {
            if let VMStackObject::VMObject(obj_ref) = &mut stack[i] {
                self.offline_if_not_variable(obj_ref);
            }
        }
        stack.truncate(pointer.unwrap_or(0));
        Ok(())
    }

    pub fn let_var(
        &mut self,
        name: String,
        value: GCRef,
        wrap: bool,
        gc_system: &mut GCSystem,
    ) -> Result<(), ContextError> {
        if let Some((vars, _, _, _)) = self.frames.last_mut() {
            if vars.contains_key(&name) {
                let var = vars.get(&name).unwrap();
                var.offline();
                vars.insert(name.clone(), value.clone());
                return Ok(());
            }
            let wrapped_value = if wrap {
                gc_system.new_object(VMVariableWrapper::new(value))
            } else {
                value
            };
            vars.insert(name, wrapped_value);
            Ok(())
        } else {
            return Err(ContextError::NoFrame);
        }
    }

    pub fn get_var(&self, name: &str) -> Result<GCRef, ContextError> {
        for (vars, _, _, _) in self.frames.iter().rev() {
            if let Some(value) = vars.get(name) {
                return Ok(value.clone());
            }
        }
        Err(ContextError::NoVariable(name.to_string()))
    }

    pub fn set_var(&mut self, name: &str, value: GCRef) -> Result<(), ContextError> {
        for (vars, _, _, _) in self.frames.iter_mut().rev() {
            if let Some(var) = vars.get_mut(name) {
                try_assign_as_vmobject(var.clone(), value.clone())
                    .map_err(|e| ContextError::VMVariableError(e))?;
            }
        }
        Err(ContextError::NoVariable(name.to_string()))
    }

pub fn format_context(&self, stack: &Vec<VMStackObject>) -> String {
    use colored::*;
    
    let mut output = String::new();
    // Format stack frames
    if self.frames.is_empty() {
        output.push_str(&"No active stack frames\n\n".yellow().to_string());
    } else {
        output.push_str(&"=== Stack Frames ===\n\n".bright_blue().bold().to_string());
        
        for (i, (vars, is_function_frame, function_code_position, is_hidden_frame)) in self.frames.iter().enumerate().rev() {
            // Frame header
            let frame_type = if *is_function_frame { "Function Frame" } else { "Normal Frame" };
            let hidden_status = if *is_hidden_frame { " (Hidden)" } else { "" };
            
            let frame_header = format!("Frame #{} - {}{}\n", i, frame_type, hidden_status);
            output.push_str(&frame_header.green().bold().to_string());
            
            if *is_function_frame {
                let position_info = format!("Function code position: {}\n", function_code_position);
                output.push_str(&position_info.cyan().to_string());
            }
            
            // Variables list
            if vars.is_empty() {
                output.push_str(&"  No variables in this frame\n\n".yellow().to_string());
            } else {
                output.push_str(&"  Variables:\n".bright_magenta().to_string());
                
                for (name, var) in vars.iter() {
                    let var_value = try_repr_vmobject(var.clone(), None)
                        .unwrap_or_else(|_| format!("<cannot display>"));
                    
                    let variable_line = format!("    - {} = {}\n", 
                        name.bright_yellow(), var_value.bright_white());
                    output.push_str(&variable_line);
                }
                output.push_str("\n");
            }
        }
    }
    
    // Format stack contents
    output.push_str(&"=== Stack Contents ===\n\n".bright_blue().bold().to_string());
    
    if stack.is_empty() {
        output.push_str(&"Stack is empty\n\n".yellow().to_string());
    } else {
        for (i, item) in stack.iter().enumerate() {
            match item {
                VMStackObject::LastIP(self_lambda, ip, is_function_call) => {
                    let self_lambda_value = try_repr_vmobject(self_lambda.clone(), None)
                        .unwrap_or_else(|_| format!("<cannot display>"));
                    let call_type = if *is_function_call { "function call" } else { "normal jump" };
                    let ip_line = format!("+ [{}][{}] Instruction Pointer: {} ({})\n", 
                        i, self_lambda_value, ip, call_type);
                    output.push_str(&ip_line.cyan().to_string());
                },
                VMStackObject::VMObject(obj_ref) => {
                    let obj_value = try_repr_vmobject(obj_ref.clone(), None)
                        .unwrap_or_else(|_| format!("<cannot display>"));
                    
                    let object_line = format!("+ [{}] {}\n", i, obj_value);
                    output.push_str(&object_line.bright_white().to_string());
                }
            }
        }
        output.push_str("\n");
    }
    
    // Add stack pointers information
    output.push_str(&"=== Stack Pointers ===\n\n".bright_blue().bold().to_string());
    if self.stack_pointers.is_empty() {
        output.push_str(&"No active stack pointers\n".yellow().to_string());
    } else {
        for (i, ptr) in self.stack_pointers.iter().enumerate() {
            let pointer_line = format!("Frame #{}: Position {}\n", i, ptr);
            output.push_str(&pointer_line.bright_green().to_string());
        }
    }
    
    output
}
    pub fn debug_print_all_vars(&self) {
        for (vars, _, _, _) in self.frames.iter().rev() {
            for (name, var) in vars.iter() {
                println!("{}: {:?}, refs: {:?}", name, try_repr_vmobject(var.clone(), None), var.get_traceable().references);
            }
        }
    }

}
