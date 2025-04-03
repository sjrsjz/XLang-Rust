use rustc_hash::FxHashMap as HashMap;

use super::super::gc::gc::GCRef;
use super::super::gc::gc::GCSystem;
use super::variable::try_assign_as_vmobject;
use super::variable::try_repr_vmobject;
use super::variable::VMStackObject;
use super::variable::VMVariableError;

#[derive(Debug, PartialEq, Eq)]
pub enum ContextFrameType {
    FunctionFrame, // 函数帧
    NormalFrame,   // 普通帧
    BoundaryFrame,      // 边界帧
}

#[derive(Debug)]
pub struct Context {
    pub(crate) frames: Vec<(HashMap<String, GCRef>, ContextFrameType, usize, bool)>, // vars, is_function_frame, function_code_position, is_hidden_frame
    pub(crate) stack_pointers: Vec<usize>,
}

#[derive(Debug)]
pub enum ContextError {
    NoFrame(ContextFrameType),
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
            ContextError::NoFrame(frame_type) => format!(
                "No frame: {:?}",
                match frame_type {
                    ContextFrameType::FunctionFrame => "Function Frame",
                    ContextFrameType::NormalFrame => "Normal Frame",
                    ContextFrameType::BoundaryFrame => "Boundary Frame",
                }
            ),
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

impl Default for Context {
    fn default() -> Self {
        Self::new()
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
        frame_type: ContextFrameType,
        function_code_position: usize,
        is_hidden_frame: bool,
    ) {
        self.frames.push((
            HashMap::default(),
            frame_type,
            function_code_position,
            is_hidden_frame,
        ));
        self.stack_pointers.push(stack.len());
    }

    pub fn pop_frame_until_function(
        &mut self,
        stack: &mut Vec<VMStackObject>,
        instructions: &mut Vec<GCRef>,
    ) -> Result<(), ContextError> {
        while !self.frames.is_empty()
            && self.frames.last().unwrap().1 != ContextFrameType::FunctionFrame
        {
            self.pop_frame_except_top(stack, instructions)?;
        }
        if self.frames.is_empty() {
            return Err(ContextError::NoFrame(ContextFrameType::FunctionFrame));
        }
        // 处理函数帧的情况
        self.pop_frame_except_top(stack, instructions)?;
        Ok(())
    }

    pub fn pop_frame_until_boundary(
        &mut self,
        stack: &mut Vec<VMStackObject>,
        instructions: &mut Vec<GCRef>,
    ) -> Result<(), ContextError> {
        while !self.frames.is_empty()
            && self.frames.last().unwrap().1 != ContextFrameType::BoundaryFrame
        {
            self.pop_frame_except_top(stack, instructions)?;
        }
        if self.frames.is_empty() {
            return Err(ContextError::NoFrame(ContextFrameType::BoundaryFrame));
        }
        // 处理边界帧的情况
        self.pop_frame_except_top(stack, instructions)?;
        Ok(())
    }

    pub fn pop_frame_except_top(
        &mut self,
        stack: &mut Vec<VMStackObject>,
        instructions: &mut Vec<GCRef>
    ) -> Result<(), ContextError> {
        // 处理单个帧弹出的情况
        if self.frames.is_empty() {
            return Err(ContextError::NoFrame(ContextFrameType::NormalFrame));
        }

        for variable in self.frames.last_mut().unwrap().0.values_mut() {
            variable.drop_ref();
        }
        // 2. 保存栈指针
        let stack_pointer = self.stack_pointers.pop().unwrap_or(0);

        // 3. 安全地更新帧和栈指针
        self.frames.pop();

        // 4. 离线变量（在数据结构已更新后）

        // 5. 处理栈上的对象
        for i in stack_pointer..stack.len() - 1 {
            match &mut stack[i] {
                VMStackObject::VMObject(obj_ref) => {
                    obj_ref.drop_ref();
                }
                VMStackObject::LastIP(self_lambda, _ip, use_new_instructions) => {
                    // 这里的self_lambda是一个函数对象，可能会被GC回收
                    self_lambda.drop_ref();
                    if *use_new_instructions {
                        let mut poped = instructions.pop();
                        if let Some(instruction) = poped.as_mut() {
                            // 这里的instruction是一个函数对象，可能会被GC回收
                            instruction.drop_ref();
                        } else {
                            return Err(ContextError::ContextError(
                                "No instructions left in the stack".to_string(),
                            ));
                        }
                    }
                }
            }
            
        }

        // 6. 截断栈（但不删除最后一个元素）
        if stack.len() == 0 {
            return Err(ContextError::ContextError("Empty stack".to_string()));
        }
        if stack_pointer >= stack.len(){
            return Err(ContextError::ContextError("Stack pointer out of bounds".to_string()));
        }
        stack[stack_pointer] = stack[stack.len() - 1].clone();
        stack.truncate(stack_pointer + 1);

        Ok(())
    }

    pub fn let_var(
        &mut self,
        name: &str,
        value: &mut GCRef,
        _gc_system: &mut GCSystem,
    ) -> Result<(), ContextError> {
        if let Some((vars, _, _, _)) = self.frames.last_mut() {
            if let Some(existing_var) = vars.get_mut(name) {
                let mut old = existing_var.clone();
                *existing_var = value.clone_ref();
                old.drop_ref(); // 扔掉旧的引用，因为已经被覆盖了
                return Ok(());
            }
            vars.insert(name.to_string(), value.clone_ref());
            Ok(())
        } else {
            Err(ContextError::NoFrame(ContextFrameType::NormalFrame))
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
                try_assign_as_vmobject(var, value).map_err(ContextError::VMVariableError)?;
            }
        }
        Err(ContextError::NoVariable(name.to_string()))
    }
    pub fn format_context(&self, stack: &Vec<VMStackObject>) -> String {
        use colored::*;

        // 定义统一的颜色主题
        let title_color = Color::BrightBlue; // 所有主标题
        let section_color = Color::Blue; // 所有区块标题
        let function_color = Color::Green; // 函数相关
        let normal_color = Color::Cyan; // 普通流程相关
        let variable_color = Color::Yellow; // 变量名
        let value_color = Color::BrightWhite; // 值显示

        let mut output = String::new();
        // Format stack frames
        if self.frames.is_empty() {
            output.push_str(&"No active stack frames\n\n".dimmed().to_string());
        } else {
            output.push_str(
                &"=== Stack Frames ===\n\n"
                    .to_string()
                    .color(title_color)
                    .bold()
                    .to_string(),
            );

            for (i, (vars, frame_type, function_code_position, is_hidden_frame)) in
                self.frames.iter().enumerate().rev()
            {
                // Frame header
                let frame_type_str = match frame_type {
                    ContextFrameType::FunctionFrame => "Function Frame",
                    ContextFrameType::NormalFrame => "Normal Frame",
                    ContextFrameType::BoundaryFrame => "Boundary Frame",
                };
                let hidden_status = if *is_hidden_frame { " (Hidden)" } else { "" };

                // 使用统一颜色区分函数帧和普通帧
                let frame_header = if *is_hidden_frame {
                    format!("Frame #{} - {}{}\n", i, frame_type_str, hidden_status)
                        .dimmed()
                        .to_string()
                } else if *frame_type == ContextFrameType::FunctionFrame {
                    format!("Frame #{} - {}{}\n", i, frame_type_str, hidden_status)
                        .color(function_color)
                        .bold()
                        .to_string()
                } else if *frame_type == ContextFrameType::BoundaryFrame {
                    format!("Frame #{} - {}{}\n", i, frame_type_str, hidden_status)
                        .color(section_color)
                        .bold()
                        .to_string()
                } else {
                    format!("Frame #{} - {}{}\n", i, frame_type_str, hidden_status)
                        .color(normal_color)
                        .bold()
                        .to_string()
                };
                output.push_str(&frame_header);

                if *frame_type == ContextFrameType::FunctionFrame {
                    let position_info =
                        format!("Function code position: {}\n", function_code_position);
                    output.push_str(&position_info.color(function_color).to_string());
                }

                // Variables list
                if vars.is_empty() {
                    output.push_str(&"  No variables in this frame\n\n".dimmed().to_string());
                } else {
                    output.push_str(&"  Variables:\n".color(section_color).bold().to_string());

                    for (name, var) in vars.iter() {
                        let var_value = try_repr_vmobject(var.clone(), None)
                            .unwrap_or_else(|_| "<cannot display>".to_string());

                        // 统一变量名和值的颜色
                        let variable_line = format!(
                            "    - {} = {}\n",
                            name.color(variable_color).bold(),
                            var_value.color(value_color)
                        );
                        output.push_str(&variable_line);
                    }
                    output.push('\n');
                }
            }
        }

        // Format stack contents
        output.push_str(
            &"=== Stack Contents ===\n\n"
                .to_string()
                .color(title_color)
                .bold()
                .to_string(),
        );

        if stack.is_empty() {
            output.push_str(&"Stack is empty\n\n".dimmed().to_string());
        } else {
            for (i, item) in stack.iter().enumerate() {
                match item {
                    VMStackObject::LastIP(self_lambda, ip, is_function_call) => {
                        let self_lambda_value = try_repr_vmobject(self_lambda.clone(), None)
                            .unwrap_or_else(|_| "<cannot display>".to_string());
                        let call_type = if *is_function_call {
                            "function call"
                        } else {
                            "normal jump"
                        };

                        // 统一函数调用和跳转的颜色
                        let (symbol, symbol_color) = if *is_function_call {
                            ("->", function_color)
                        } else {
                            (">", normal_color)
                        };

                        let ip_line = format!(
                            "{} [{}][{}] IP: {} ({})\n",
                            symbol, i, self_lambda_value, ip, call_type
                        );
                        output.push_str(&ip_line.color(symbol_color).to_string());
                    }
                    VMStackObject::VMObject(obj_ref) => {
                        let obj_value = try_repr_vmobject(obj_ref.clone(), None)
                            .unwrap_or_else(|_| "<cannot display>".to_string());

                        let object_line = format!("+ [{}] {}\n", i, obj_value);
                        output.push_str(&object_line.color(value_color).to_string());
                    }
                }
            }
            output.push('\n');
        }

        // Add stack pointers information
        output.push_str(
            &"=== Stack Pointers ===\n\n"
                .to_string()
                .color(title_color)
                .bold()
                .to_string(),
        );
        if self.stack_pointers.is_empty() {
            output.push_str(&"No active stack pointers\n".dimmed().to_string());
        } else {
            for (i, ptr) in self.stack_pointers.iter().enumerate() {
                let pointer_line = format!("Frame #{}: Position {} {}\n", i, ptr, "<-");
                output.push_str(&pointer_line.color(function_color).to_string());
            }
        }

        output
    }
    pub fn debug_print_all_vars(&self) {
        for (vars, _, _, _) in self.frames.iter().rev() {
            println!("=== Frame Variables === {}", vars.len());
            for (name, var) in vars.iter() {
                println!(
                    "{}({}): {:?}, refs: {:?} <- {}",
                    name,
                    var.clone(),
                    try_repr_vmobject(var.clone(), None),
                    var.get_const_traceable().references,
                    var.get_const_traceable().native_gcref_object_count
                );
            }
        }
    }
}
