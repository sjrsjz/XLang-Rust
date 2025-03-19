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
                try_repr_vmobject(obj.clone()).unwrap_or(format!("{:?}", obj))
            ),
            ContextError::InvalidContextVariable(obj) => format!(
                "Invalid context variable: {:?}",
                try_repr_vmobject(obj.clone()).unwrap_or(format!("{:?}", obj))
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

    pub fn slice_frames_and_stack(
        &mut self,
        stack: &mut Vec<VMStackObject>,
        size: usize,
    ) -> Result<(), ContextError> {
        // 情况1: 所有帧都被移除
        if size == 0 {
            // 离线所有帧中的变量
            for (vars, _, _, _) in &mut self.frames {
                for variable in vars.values() {
                    variable.offline();
                }
            }

            // 离线栈中的所有对象
            for stack_obj in stack.iter_mut() {
                if let VMStackObject::VMObject(obj_ref) = stack_obj {
                    self.offline_if_not_variable(obj_ref);
                }
            }

            self.frames.clear();
            self.stack_pointers.clear();
            stack.clear();
            return Ok(());
        }

        // 检查请求的大小是否合法
        if self.frames.len() < size {
            return Err(ContextError::ContextError(format!(
                "无法截断上下文：请求大小({})大于当前大小({})",
                size,
                self.frames.len()
            )));
        }

        // 情况2: 部分帧被移除
        if size < self.frames.len() {
            // 获取将保留的最后一帧的栈指针
            let stack_pointer = if size > 0 {
                self.stack_pointers[size - 1]
            } else {
                0
            };

            // 离线所有要移除的帧中的变量
            for i in size..self.frames.len() {
                let (vars, _, _, _) = &mut self.frames[i];
                for variable in vars.values() {
                    variable.offline();
                }
            }

            // 离线栈中将被移除的对象
            if stack.len() > stack_pointer {
                for stack_obj in stack.iter_mut().skip(stack_pointer) {
                    if let VMStackObject::VMObject(obj_ref) = stack_obj {
                        self.offline_if_not_variable(obj_ref);
                    }
                }
            }

            // 截断帧和栈指针
            self.frames.truncate(size);
            self.stack_pointers.truncate(size);

            // 截断数据栈
            stack.truncate(stack_pointer);
        }

        Ok(())
    }

    pub fn debug_print_all_vars(&self) {
        for (vars, _, _, _) in self.frames.iter().rev() {
            for (name, var) in vars.iter() {
                println!("{}: {:?}, refs: {:?}", name, try_repr_vmobject(var.clone()), var.get_traceable().references);
            }
        }
    }

}
