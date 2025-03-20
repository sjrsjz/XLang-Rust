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
        let mut output = String::new();
        // Format stack frames
        if self.frames.is_empty() {
            output.push_str("No active stack frames\n\n");
        } else {
            output.push_str("=== Stack Frames ===\n\n");
            
            for (i, (vars, is_function_frame, function_code_position, is_hidden_frame)) in self.frames.iter().enumerate().rev() {
                // Frame header
                let frame_type = if *is_function_frame { "Function Frame" } else { "Normal Frame" };
                let hidden_status = if *is_hidden_frame { " (Hidden)" } else { "" };
                
                output.push_str(&format!("Frame #{} - {}{}\n", i, frame_type, hidden_status));
                
                if *is_function_frame {
                    output.push_str(&format!("Function code position: {}\n", function_code_position));
                }
                
                // Variables list
                if vars.is_empty() {
                    output.push_str("  No variables in this frame\n\n");
                } else {
                    output.push_str("  Variables:\n");
                    
                    for (name, var) in vars.iter() {
                        let var_value = try_repr_vmobject(var.clone(), None)
                            .unwrap_or_else(|_| format!("<cannot display>"));
                        
                        output.push_str(&format!("    - {} = {}\n", 
                            name, var_value));
                    }
                    output.push_str("\n");
                }
                output.push_str("  ---\n\n");
            }
        }
        
        // Format stack contents
        output.push_str("=== Stack Contents ===\n\n");
        
        if stack.is_empty() {
            output.push_str("Stack is empty\n\n");
        } else {
            for (i, item) in stack.iter().enumerate() {
                match item {
                    VMStackObject::LastIP(ip, is_function_call) => {
                        let call_type = if *is_function_call { "function call" } else { "normal jump" };
                        output.push_str(&format!("+ [{}] Instruction Pointer: {} ({})\n", 
                            i, ip, call_type));
                    },
                    VMStackObject::VMObject(obj_ref) => {
                        let obj_value = try_repr_vmobject(obj_ref.clone(), None)
                            .unwrap_or_else(|_| format!("<cannot display>"));
                        
                        output.push_str(&format!("+ [{}] {}\n", 
                            i, obj_value));
                    }
                }
            }
            output.push_str("\n");
        }
        
        // Add stack pointers information
        output.push_str("=== Stack Pointers ===\n\n");
        if self.stack_pointers.is_empty() {
            output.push_str("No active stack pointers\n");
        } else {
            for (i, ptr) in self.stack_pointers.iter().enumerate() {
                output.push_str(&format!("Frame #{}: Position {}\n", i, ptr));
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
