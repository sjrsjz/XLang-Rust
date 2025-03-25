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
    // 先处理函数调用退出的情况
    if exit_function {
        while self.frames.len() > 0 && !self.frames[self.frames.len() - 1].1 {
            // 1. 先复制要离线的变量
            let mut variables_to_offline: Vec<GCRef> = if !self.frames.is_empty() {
                self.frames.last().unwrap().0.values().cloned().collect()
            } else {
                Vec::new()
            };
            
            // 2. 保存需要处理的栈范围
            let stack_range_start = self.stack_pointers.last().cloned().unwrap_or(0);
            let stack_range_end = stack.len();
            
            // 3. 安全地更新帧和栈指针
            if !self.frames.is_empty() {
                self.frames.pop();
            }
            self.stack_pointers.pop();
            
            // 4. 离线变量（在数据结构已更新后）
            for variable in variables_to_offline.iter_mut() {
                variable.drop_ref();
            }
            
            // 5. 处理栈上的对象
            for i in stack_range_start..stack_range_end {
                if let VMStackObject::VMObject(obj_ref) = &mut stack[i] {
                    obj_ref.drop_ref();
                }
            }
            
            // 6. 截断栈
            stack.truncate(stack_range_start);
        }
    }
    
    // 处理单个帧弹出的情况
    if self.frames.is_empty() {
        return Err(ContextError::NoFrame);
    }
    
    // 1. 收集当前帧的变量
    let mut variables_to_offline: Vec<GCRef> = self.frames.last().unwrap().0.values().cloned().collect();
    
    // 2. 保存栈指针
    let stack_pointer = self.stack_pointers.last().cloned().unwrap_or(0);
    
    // 3. 安全地更新帧和栈指针
    self.frames.pop();
    self.stack_pointers.pop();
    
    // 4. 离线变量（在数据结构已更新后）
    for variable in variables_to_offline.iter_mut() {
        variable.drop_ref();
    }
    
    // 5. 处理栈上的对象
    for i in stack_pointer..stack.len() {
        if let VMStackObject::VMObject(obj_ref) = &mut stack[i] {
            obj_ref.drop_ref();
        }
    }
    
    // 6. 截断栈
    stack.truncate(stack_pointer);
    
    Ok(())
}

    pub fn let_var(
        &mut self,
        name: String,
        value: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<(), ContextError> {
        if let Some((vars, _, _, _)) = self.frames.last_mut() {
            if vars.contains_key(&name) {
                let mut var = vars.get(&name).unwrap().clone();
                vars.insert(name.clone(), gc_system.new_object(VMVariableWrapper::new(value)));
                var.drop_ref(); // 扔掉旧的引用，因为已经被覆盖了
                return Ok(());
            }

            vars.insert(name, gc_system.new_object(VMVariableWrapper::new(value)));
            Ok(())
        } else {
            return Err(ContextError::NoFrame);
        }
    }

    pub fn get_var(&mut self, name: &str) -> Result<GCRef, ContextError> {
        for (vars, _, _, _) in self.frames.iter_mut().rev() {
            if let Some(value) = vars.get_mut(name) {
                return Ok(value.clone_ref()); // 这里需要clone_ref，因为我们要返回一个新的引用
            }
        }
        Err(ContextError::NoVariable(name.to_string()))
    }

    pub fn set_var(&mut self, name: &str, value: &mut GCRef) -> Result<(), ContextError> {
        for (vars, _, _, _) in self.frames.iter_mut().rev() {
            if let Some(var) = vars.get_mut(name) {
                try_assign_as_vmobject(var, value)
                    .map_err(|e| ContextError::VMVariableError(e))?;
            }
        }
        Err(ContextError::NoVariable(name.to_string()))
    }
    pub fn format_context(&self, stack: &Vec<VMStackObject>) -> String {
        use colored::*;
        
        // 定义统一的颜色主题
        let title_color = Color::BrightBlue;       // 所有主标题
        let section_color = Color::Blue;           // 所有区块标题
        let function_color = Color::Green;         // 函数相关
        let normal_color = Color::Cyan;            // 普通流程相关
        let variable_color = Color::Yellow;        // 变量名
        let value_color = Color::BrightWhite;      // 值显示
        
        let mut output = String::new();
        // Format stack frames
        if self.frames.is_empty() {
            output.push_str(&"No active stack frames\n\n".dimmed().to_string());
        } else {
            output.push_str(&format!("=== Stack Frames ===\n\n").color(title_color).bold().to_string());
            
            for (i, (vars, is_function_frame, function_code_position, is_hidden_frame)) in self.frames.iter().enumerate().rev() {
                // Frame header
                let frame_type = if *is_function_frame { "Function Frame" } else { "Normal Frame" };
                let hidden_status = if *is_hidden_frame { " (Hidden)" } else { "" };
                
                // 使用统一颜色区分函数帧和普通帧
                let frame_header = if *is_hidden_frame {
                    format!("Frame #{} - {}{}\n", i, frame_type, hidden_status).dimmed().to_string()
                } else if *is_function_frame {
                    format!("Frame #{} - {}{}\n", i, frame_type, hidden_status).color(function_color).bold().to_string()
                } else {
                    format!("Frame #{} - {}{}\n", i, frame_type, hidden_status).color(normal_color).bold().to_string()
                };
                output.push_str(&frame_header);
                
                if *is_function_frame {
                    let position_info = format!("Function code position: {}\n", function_code_position);
                    output.push_str(&position_info.color(function_color).to_string());
                }
                
                // Variables list
                if vars.is_empty() {
                    output.push_str(&"  No variables in this frame\n\n".dimmed().to_string());
                } else {
                    output.push_str(&"  Variables:\n".color(section_color).bold().to_string());
                    
                    for (name, var) in vars.iter() {
                        let var_value = try_repr_vmobject(var.clone(), None)
                            .unwrap_or_else(|_| format!("<cannot display>"));
                        
                        // 统一变量名和值的颜色
                        let variable_line = format!("    - {} = {}\n", 
                            name.color(variable_color).bold(), var_value.color(value_color));
                        output.push_str(&variable_line);
                    }
                    output.push_str("\n");
                }
            }
        }
        
        // Format stack contents
        output.push_str(&format!("=== Stack Contents ===\n\n").color(title_color).bold().to_string());
        
        if stack.is_empty() {
            output.push_str(&"Stack is empty\n\n".dimmed().to_string());
        } else {
            for (i, item) in stack.iter().enumerate() {
                match item {
                    VMStackObject::LastIP(self_lambda, ip, is_function_call) => {
                        let self_lambda_value = try_repr_vmobject(self_lambda.clone(), None)
                            .unwrap_or_else(|_| format!("<cannot display>"));
                        let call_type = if *is_function_call { "function call" } else { "normal jump" };
                        
                        // 统一函数调用和跳转的颜色
                        let (symbol, symbol_color) = if *is_function_call { 
                            ("->", function_color) 
                        } else { 
                            (">", normal_color) 
                        };
                        
                        let ip_line = format!("{} [{}][{}] IP: {} ({})\n", 
                            symbol, i, self_lambda_value, ip, call_type);
                        output.push_str(&ip_line.color(symbol_color).to_string());
                    },
                    VMStackObject::VMObject(obj_ref) => {
                        let obj_value = try_repr_vmobject(obj_ref.clone(), None)
                            .unwrap_or_else(|_| format!("<cannot display>"));
                        
                        let object_line = format!("+ [{}] {}\n", i, obj_value);
                        output.push_str(&object_line.color(value_color).to_string());
                    }
                }
            }
            output.push_str("\n");
        }
        
        // Add stack pointers information
        output.push_str(&format!("=== Stack Pointers ===\n\n").color(title_color).bold().to_string());
        if self.stack_pointers.is_empty() {
            output.push_str(&"No active stack pointers\n".dimmed().to_string());
        } else {
            for (i, ptr) in self.stack_pointers.iter().enumerate() {
                let pointer_line = format!("Frame #{}: Position {} {}\n", 
                    i, ptr, "<-");
                output.push_str(&pointer_line.color(function_color).to_string());
            }
        }
        
        output
    }
    pub fn debug_print_all_vars(&self) {
        for (vars, _, _, _) in self.frames.iter().rev() {
            for (name, var) in vars.iter() {
                println!("{}: {:?}, refs: {:?}", name, try_repr_vmobject(var.clone(), None), var.get_const_traceable().references);
            }
        }
    }

}
