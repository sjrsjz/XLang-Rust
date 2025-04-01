use rustc_hash::FxHashMap as HashMap;

use crate::vm::ir::DebugInfo;
use crate::vm::ir::IR;

use super::super::gc::gc::*;
use super::context::*;
use super::variable::*;

#[derive(Debug)]
pub enum VMError {
    InvalidInstruction(IR),
    TryEnterNotLambda(GCRef),
    EmptyStack,
    ArgumentIsNotTuple(GCRef),
    UnableToReference(GCRef),
    NotVMObject(VMStackObject),
    ContextError(ContextError),
    VMVariableError(VMVariableError),
    AssertFailed,
    CannotGetSelf(GCRef),
    InvalidArgument(GCRef, String),
    FileError(String),
    DetailedError(String),
}

impl VMError {
    pub fn to_string(&self) -> String {
        use colored::*;

        match self {
            VMError::InvalidInstruction(instruction) => {
                format!(
                    "{}: {:?}",
                    "InvalidInstruction".bright_red().bold(),
                    instruction
                )
            }
            VMError::TryEnterNotLambda(lambda) => format!(
                "{}: {}",
                "TryEnterNotLambda".bright_red().bold(),
                try_repr_vmobject(lambda.clone(), None).unwrap_or(format!("{:?}", lambda))
            ),
            VMError::EmptyStack => "EmptyStack".bright_red().bold().to_string(),
            VMError::ArgumentIsNotTuple(tuple) => format!(
                "{}: {}",
                "ArgumentIsNotTuple".bright_red().bold(),
                try_repr_vmobject(tuple.clone(), None).unwrap_or(format!("{:?}", tuple))
            ),
            VMError::UnableToReference(obj) => format!(
                "{}: {}",
                "UnableToReference".bright_red().bold(),
                try_repr_vmobject(obj.clone(), None).unwrap_or(format!("{:?}", obj))
            ),
            VMError::NotVMObject(obj) => {
                format!("{}: {:?}", "NotVMObject".bright_red().bold(), obj)
            }
            VMError::ContextError(err) => format!(
                "{}: {}",
                "ContextError".bright_red().bold(),
                err.to_string()
            ),
            VMError::VMVariableError(err) => format!(
                "{}: {}",
                "VMVariableError".bright_red().bold(),
                err.to_string()
            ),
            VMError::AssertFailed => "AssertFailed".bright_red().bold().to_string(),
            VMError::CannotGetSelf(obj) => format!(
                "{}: {}",
                "CannotGetSelf".bright_red().bold(),
                try_repr_vmobject(obj.clone(), None).unwrap_or(format!("{:?}", obj))
            ),
            VMError::InvalidArgument(obj, msg) => format!(
                "{}: {} {}",
                "InvalidArgument".bright_red().bold(),
                try_repr_vmobject(obj.clone(), None).unwrap_or(format!("{:?}", obj)),
                format!("because {}", msg).bright_red()
            ),
            VMError::FileError(msg) => format!("{}: {}", "FileError".bright_red().bold(), msg),
            VMError::DetailedError(msg) => msg.to_string(),
        }
    }
}

#[derive(Debug)]
// 协程池
pub struct VMCoroutinePool {
    executors: Vec<(IRExecutor, isize)>, // executor, id
    gen_id: isize,
    enable_dump: bool,
}

impl VMCoroutinePool {
    pub fn new(enable_dump: bool) -> Self {
        VMCoroutinePool {
            executors: Vec::new(),
            gen_id: 0,
            enable_dump,
        }
    }

    /**
     * 创建一个新的协程
     * lambda_object: 协程对象
     * original_code: 原始代码
     * gc_system: 垃圾回收系统
     * 返回协程的 ID
     *
     * 注意：
     * + 如果协程对象已经在执行器中，则会返回错误
     * + lambda_object 必须是 VMLambda 类型
     * + original_code 是可选的，如果提供，则会在调试信息中使用
     * + 启动协程会消耗一个native_gcref_object_count(通过 drop_ref())
     */
    pub fn new_coroutine(
        &mut self,
        lambda_object: &mut GCRef,
        original_code: Option<String>,
        gc_system: &mut GCSystem,
    ) -> Result<isize, VMError> {
        if !lambda_object.isinstance::<VMLambda>() {
            return Err(VMError::DetailedError(
                "lambda_object must be a VMLambda".to_string(),
            ));
        }
        let mut executor = IRExecutor::new(&lambda_object.clone_ref(), original_code);

        // 检查是否已有执行器使用该 lambda
        for (executor, _) in &self.executors {
            // 比较两个 lambda 是否是同一个对象
            if std::ptr::eq(
                executor.entry_lambda.get_const_reference() as *const _,
                lambda_object.get_const_reference() as *const _,
            ) {
                return Err(VMError::DetailedError(
                    "Attempted to start the same lambda coroutine multiple times".to_string(),
                ));
            }
        }

        executor.init(lambda_object, gc_system)?;
        self.executors.push((executor, self.gen_id));
        let id = self.gen_id;
        self.gen_id += 1;

        lambda_object.drop_ref();
        Ok(id)
    }

    pub fn get_coroutine(&self, id: isize) -> Option<&IRExecutor> {
        for (executor, executor_id) in &self.executors {
            if *executor_id == id {
                return Some(executor);
            }
        }
        None
    }

    pub fn step_all(
        &mut self,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, (isize, VMError)> {
        let mut spawned_coroutines = Vec::<SpawnedCoroutine>::new();
        for (e, id) in &mut self.executors {
            let new_coroutines = e.step(gc_system);
            if new_coroutines.is_err() {
                e.entry_lambda.as_type::<VMLambda>().coroutine_status = VMCoroutineStatus::Crashed;
                return Err((*id, new_coroutines.err().unwrap()));
            }
            let new_coroutines = new_coroutines.unwrap();
            if let Some(new_coroutines) = new_coroutines {
                spawned_coroutines.extend(new_coroutines);
            }
        }

        gc_system.check_and_collect();

        Ok(Some(spawned_coroutines))
    }

    pub fn sweep_finished(&mut self) {
        // 第一阶段：收集已完成的协程索引
        let mut finished_indices = Vec::new();
        for (i, (executor, _id)) in self.executors.iter().enumerate() {
            if executor
                .entry_lambda
                .as_const_type::<VMLambda>()
                .coroutine_status
                == VMCoroutineStatus::Finished
            {
                finished_indices.push(i);
            }
        }

        // 第二阶段：从后向前移除已完成的协程
        for &idx in finished_indices.iter().rev() {
            if idx < self.executors.len() {
                // 安全地移除和释放资源
                let (mut executor, _) = self.executors.remove(idx);
                executor.entry_lambda.drop_ref();
            }
        }
    }

    pub fn run_until_finished(&mut self, gc_system: &mut GCSystem) -> Result<(), VMError> {
        use colored::*;

        loop {
            let spawned_coroutines = self.step_all(gc_system).map_err(|vm_error| {
                if self.enable_dump {
                    let all_coroutines_contexts_repr = self
                        .executors
                        .iter_mut()
                        .map(|(e, _)| {
                            let lambda = e.entry_lambda.as_const_type::<VMLambda>();
                            format!(
                                "{}\n{}\n\n{}\n\n{}",
                                format!(
                                    "-> {}: {}",
                                    lambda.signature,
                                    lambda.coroutine_status.to_string()
                                )
                                .bright_yellow()
                                .bold(),
                                e.context.format_context(&e.stack),
                                "=== Code ===".bright_blue().bold(),
                                e.repr_current_code(Some(2))
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n\n");

                    VMError::DetailedError(format!(
                        "{}\n\n{}\n{}\n\n{}",
                        "** CoroutinePool Step Error! **".bright_red().bold(),
                        "# Main Error".bright_red().bold().underline(),
                        vm_error.1.to_string().red(),
                        format!("All Coroutine Contexts:\n{}", all_coroutines_contexts_repr)
                    ))
                } else {
                    vm_error.1
                }
            })?;

            self.sweep_finished();

            if let Some(mut coroutines) = spawned_coroutines {
                for coroutine in coroutines.iter_mut() {
                    let lambda_object = &mut coroutine.lambda_ref;
                    let source_code = coroutine.source_code.clone();
                    self.new_coroutine(lambda_object, source_code, gc_system)?;
                }
            }

            if self.executors.is_empty() {
                break;
            }
        }

        Ok(())
    }
}
#[derive(Debug)]
pub struct SpawnedCoroutine {
    lambda_ref: GCRef,
    source_code: Option<String>,
}

#[derive(Debug)]
pub struct IRExecutor {
    context: Context,
    stack: Vec<VMStackObject>,
    ip: isize,
    lambda_instructions: Vec<GCRef>,
    original_code: Option<String>,
    debug_info: Option<DebugInfo>,
    entry_lambda: GCRef,
}

mod native_functions {
    use std::io::Write;

    use crate::vm::{
        executor::variable::{
            try_repr_vmobject, VMBoolean, VMBytes, VMFloat, VMInt, VMNull, VMString, VMTuple,
            VMVariableError,
        },
        gc::gc::{GCRef, GCSystem},
    };

    fn check_if_tuple(tuple: GCRef) -> Result<(), VMVariableError> {
        if !tuple.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(
                tuple,
                "native function's input must be a tuple".to_string(),
            ));
        }
        Ok(())
    }

    pub fn print(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple = tuple.as_const_type::<VMTuple>();
        let mut result = String::new();
        for obj in &tuple.values {
            let repr = try_repr_vmobject(obj.clone(), None)?;
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
                "len function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMTuple>() {
            let inner_tuple = tuple_obj.values[0].as_const_type::<VMTuple>();
            let obj = gc_system.new_object(VMInt::new(inner_tuple.values.len() as i64));
            Ok(obj)
        } else if tuple_obj.values[0].isinstance::<VMString>() {
            let inner_string = tuple_obj.values[0].as_const_type::<VMString>();
            let obj = gc_system.new_object(VMInt::new(inner_string.value.len() as i64));
            return Ok(obj);
        } else if tuple_obj.values[0].isinstance::<VMBytes>() {
            let inner_bytes = tuple_obj.values[0].as_const_type::<VMBytes>();
            let obj = gc_system.new_object(VMInt::new(inner_bytes.value.len() as i64));
            return Ok(obj);
        } else {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "len function's input should be a string or a tuple".to_string(),
            ));
        }
    }

    pub fn to_int(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_int function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMInt::new(0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_const_type::<VMBoolean>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_int function's input should be a int".to_string(),
        ))
    }

    pub fn to_float(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_float function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMFloat::new(0.0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMBoolean>()
                .to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_float function's input should be a float".to_string(),
        ))
    }

    pub fn to_string(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_string function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMString::new("null".to_string())));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMBoolean>()
                .to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMBytes>() {
            let data = tuple_obj.values[0].as_const_type::<VMBytes>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_string function's input should be a string".to_string(),
        ))
    }

    pub fn to_bool(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_bool function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_const_type::<VMInt>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_const_type::<VMFloat>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_const_type::<VMString>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMBoolean::new(false)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_const_type::<VMBoolean>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_bool function's input should be a bool".to_string(),
        ))
    }
    pub fn to_bytes(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_bytes function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMBytes>() {
            return Ok(gc_system.new_object(VMBytes::new(
                tuple_obj.values[0].as_const_type::<VMBytes>().value.clone(),
            )));
        } else if tuple_obj.values[0].isinstance::<VMString>() {
            // 将字符串转换为字节序列
            let string_value = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .value
                .clone();
            return Ok(gc_system.new_object(VMBytes::new(string_value.as_bytes().to_vec())));
        } else if tuple_obj.values[0].isinstance::<VMInt>() {
            // 支持单字节的整数转字节
            let int_value = tuple_obj.values[0].as_const_type::<VMInt>().value;
            if !(0..=255).contains(&int_value) {
                return Err(VMVariableError::ValueError(
                    tuple_obj.values[0].clone(),
                    "Integer values for bytes conversion must be between 0 and 255".to_string(),
                ));
            }
            return Ok(gc_system.new_object(VMBytes::new(vec![int_value as u8])));
        } else if tuple_obj.values[0].isinstance::<VMTuple>() {
            // 支持整数元组转字节序列
            let inner_tuple = tuple_obj.values[0].as_const_type::<VMTuple>();
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

            return Ok(gc_system.new_object(VMBytes::new(byte_vec)));
        }

        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_bytes function's input should be a bytes, string, integer, or tuple of integers"
                .to_string(),
        ))
    }

    pub fn input(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_const_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "input function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0]
                .as_const_type::<VMString>()
                .to_string()?;
            print!("{} ", data);
            std::io::stdout().flush().unwrap_or(());
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let data = input.trim().to_string();
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "input function's input should be a string".to_string(),
        ))
    }
}

impl IRExecutor {
    pub fn new(entry_lambda: &GCRef, original_code: Option<String>) -> Self {
        IRExecutor {
            context: Context::new(),
            stack: Vec::new(),
            ip: 0,
            lambda_instructions: Vec::new(),
            original_code,
            debug_info: None,
            entry_lambda: entry_lambda.clone(),
        }
    }
    pub fn set_debug_info(&mut self, debug_info: DebugInfo) {
        self.debug_info = Some(debug_info);
    }
    pub fn repr_current_code(&self, context_lines: Option<usize>) -> String {
        use colored::*;
        use unicode_segmentation::UnicodeSegmentation;

        let context_lines = context_lines.unwrap_or(2); // Default to 2 lines of context

        if self.original_code.is_none() || self.debug_info.is_none() {
            return String::from("[Source code information not available]")
                .bright_yellow()
                .italic()
                .to_string();
        }

        let source_code = self.original_code.as_ref().unwrap();
        let debug_info = self.debug_info.as_ref().unwrap();

        // Check if current IP is out of bounds
        if self.ip < 0
            || self.ip as usize
                >= self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .instructions
                    .len()
        {
            return format!("[IP out of range: {}]", self.ip)
                .bright_red()
                .bold()
                .to_string();
        }

        // Get source position for current instruction
        let current_pos = debug_info.code_position;

        // Split source code into lines
        let lines: Vec<&str> = source_code.lines().collect();

        // Helper function to find line and column from byte position
        let find_position = |byte_pos: usize| -> (usize, usize) {
            let mut current_byte = 0;
            for (line_num, line) in lines.iter().enumerate() {
                let line_bytes = line.len() + 1; // +1 for newline
                if current_byte + line_bytes > byte_pos {
                    // 计算行内的字节偏移
                    let line_offset = byte_pos - current_byte;

                    // 找到有效的字符边界
                    let valid_offset = line
                        .char_indices()
                        .map(|(i, _)| i)
                        .take_while(|&i| i <= line_offset)
                        .last()
                        .unwrap_or(0);

                    // 使用有效的字节偏移获取文本
                    let column_text = &line[..valid_offset];
                    let column = column_text.graphemes(true).count();
                    return (line_num, column);
                }
                current_byte += line_bytes;
            }
            (lines.len() - 1, 0) // Default to last line
        };

        // Get line and column number for current position
        let (line_num, col_num) = find_position(current_pos);

        // Calculate range of lines to display
        let start_line = line_num.saturating_sub(context_lines);
        let end_line = std::cmp::min(line_num + context_lines, lines.len() - 1);

        // Build result string
        let mut result = String::new();

        // Add context lines with current line highlighted
        for i in start_line..=end_line {
            // Format line number
            let line_prefix = format!("{:4} | ", i + 1);

            // Format line content - highlight current line
            let line_content = if i == line_num {
                lines[i].bright_white().bold().to_string()
            } else {
                lines[i].white().to_string()
            };

            result.push_str(&line_prefix.bright_black().to_string());
            result.push_str(&line_content);
            result.push('\n');

            // Mark the current line with pointer
            if i == line_num {
                let mut marker = " ".repeat(line_prefix.len());

                // Calculate grapheme-aware marker position
                let prefix_graphemes = if col_num > 0 {
                    lines[i][..lines[i]
                        .grapheme_indices(true)
                        .nth(col_num.saturating_sub(1))
                        .map_or(lines[i].len(), |(idx, _)| idx)]
                        .graphemes(true)
                        .count()
                } else {
                    0
                };

                marker.push_str(&" ".repeat(prefix_graphemes));
                marker.push_str(&"^".bright_green().bold().to_string());

                result.push_str(&marker);
                result.push('\n');
            }
        }

        // Add current instruction information
        let instruction = &self
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .instructions[self.ip as usize];

        // Format current instruction info
        result.push_str(&format!(
            "{} {} {}\n",
            "Current instruction:".bright_blue().bold(),
            format!("{:?}", instruction)
                .bright_cyan()
                .bold()
                .underline(),
            format!("(IP: {})", self.ip).bright_blue().bold()
        ));

        result
    }

    pub fn pop_object(&mut self) -> Result<VMStackObject, VMError> {
        match self.stack.pop() {
            Some(obj) => Ok(obj),
            None => Err(VMError::EmptyStack),
        }
    }

    pub fn pop_object_and_check(&mut self) -> Result<GCRef, VMError> {
        let obj = self.pop_object()?;
        if let VMStackObject::VMObject(obj) = obj {
            return Ok(obj);
        }
        Err(VMError::NotVMObject(obj))
    }

    pub fn inject_builtin_functions(
        context: &mut Context,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        let mut built_in_functions: HashMap<String, GCRef> = HashMap::default();
        built_in_functions.insert(
            "print".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::print)),
        );
        built_in_functions.insert(
            "len".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::len)),
        );
        built_in_functions.insert(
            "int".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_int)),
        );
        built_in_functions.insert(
            "float".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_float)),
        );
        built_in_functions.insert(
            "string".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_string)),
        );
        built_in_functions.insert(
            "bytes".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_bytes)),
        );
        built_in_functions.insert(
            "bool".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_bool)),
        );
        built_in_functions.insert(
            "input".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::input)),
        );

        for (name, func) in built_in_functions.iter_mut() {
            let result = context.let_var(name.clone(), func, gc_system);
            func.drop_ref();
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }
        Ok(())
    }

    pub fn debug_output_stack(&self) {
        println!("Stack:");
        for (i, obj) in self.stack.iter().enumerate() {
            match obj {
                VMStackObject::VMObject(obj) => {
                    let repr = try_repr_vmobject(obj.clone(), None);
                    if repr.is_ok() {
                        println!("{}: {:?}", i, repr.unwrap());
                    } else {
                        println!("{}: {:?}", i, obj);
                    }
                }
                VMStackObject::LastIP(self_lambda, ip, use_new_instructions) => {
                    println!(
                        "{}: LastIP: {} {} {}",
                        i, self_lambda, ip, use_new_instructions
                    );
                }
            }
        }
    }
}

 mod vm_instructions {

    use std::fs::File;
    use std::io::Read;

    use crate::vm::executor::context::ContextFrameType;
    use crate::vm::executor::variable::*;
    use crate::vm::executor::vm::VMError;
    use crate::vm::gc::gc::GCSystem;
    use crate::vm::ir::{IROperation, IRPackage};
    use crate::IRExecutor;

    use super::SpawnedCoroutine;

    pub fn load_int(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        value: i64,
    ) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMInt::new(value));
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_float(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        value: f64,
    ) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMFloat::new(value));
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_string(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        value: String,
    ) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMString::new(value));
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_bool(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        value: bool,
    ) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMBoolean::new(value));
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_null(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMNull::new());
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_bytes(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        value: Vec<u8>,
    ) -> Result<(), VMError> {
        let obj = gc_system.new_object(VMBytes::new(value));
        vm.push_vmobject(obj)?;
        Ok(())
    }
    pub fn load_lambda(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        signature: String,
        code_position: usize,
    ) -> Result<(), VMError> {
        let instruction = &mut vm.pop_object_and_check()?;
        if !instruction.isinstance::<VMInstructions>() {
            return Err(VMError::InvalidArgument(
                instruction.clone(),
                "LoadLambda requires a VMInstructions".to_string(),
            ));
        }
        let default_args_tuple = &mut vm.pop_object_and_check()?;
        if !default_args_tuple.isinstance::<VMTuple>() {
            return Err(VMError::ArgumentIsNotTuple(default_args_tuple.clone()));
        }

        let mut lambda_result = gc_system.new_object(VMNull::new());

        let obj = gc_system.new_object(VMLambda::new(
            code_position,
            signature,
            default_args_tuple,
            None,
            instruction,
            &mut lambda_result,
        ));
        vm.push_vmobject(obj)?;
        default_args_tuple.drop_ref();
        lambda_result.drop_ref();
        instruction.drop_ref();
        Ok(())
    }
    pub fn fork_instruction(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let forked = vm.lambda_instructions.last_mut().unwrap().clone_ref();
        vm.push_vmobject(forked)?;
        Ok(())
    }
    pub fn build_tuple(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        size: usize,
    ) -> Result<(), VMError> {
        let mut tuple = Vec::new();
        for _ in 0..size {
            let obj = vm.pop_object_and_check()?;
            tuple.insert(0, obj);
        }
        let mut new_refs = tuple.iter_mut().collect();
        let obj = gc_system.new_object(VMTuple::new(&mut new_refs));
        vm.push_vmobject(obj)?;
        for obj in tuple.iter_mut() {
            obj.drop_ref();
        }
        Ok(())
    }
    pub fn bind_self(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        if !obj.isinstance::<VMTuple>() {
            return Err(VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Bind requires a tuple".to_string(),
            )));
        }
        let mut copied = try_copy_as_vmobject(obj, gc_system).map_err(VMError::VMVariableError)?;
        VMTuple::set_lambda_self(&mut copied);
        vm.push_vmobject(copied)?;
        obj.drop_ref();
        Ok(())
    }

    pub fn build_keyval(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let key = &mut vm.pop_object_and_check()?;
        let obj = gc_system.new_object(VMKeyVal::new(key, value));
        vm.push_vmobject(obj)?;
        key.drop_ref();
        value.drop_ref();
        Ok(())
    }
    pub fn build_named(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let key = &mut vm.pop_object_and_check()?;
        let obj = gc_system.new_object(VMNamed::new(key, value));
        vm.push_vmobject(obj)?;
        key.drop_ref();
        value.drop_ref();
        Ok(())
    }
    pub fn binary_op(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        operation: IROperation,
    ) -> Result<(), VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            match operation {
                IROperation::Equal => {
                    gc_system.new_object(VMBoolean::new(try_eq_as_vmobject(&left, &right)))
                }
                IROperation::NotEqual => {
                    gc_system.new_object(VMBoolean::new(!try_eq_as_vmobject(&left, &right)))
                }
                IROperation::Greater => {
                    let result = try_greater_than_as_vmobject(&left, &right)
                        .map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(result))
                }
                IROperation::Less => {
                    let result = try_less_than_as_vmobject(&left, &right)
                        .map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(result))
                }
                IROperation::GreaterEqual => {
                    let result = try_less_than_as_vmobject(&left, &right)
                        .map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(!result))
                }
                IROperation::LessEqual => {
                    let result = try_greater_than_as_vmobject(&left, &right)
                        .map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(!result))
                }

                IROperation::Add => try_add_as_vmobject(&mut left, &mut right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::Subtract => try_sub_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::Multiply => try_mul_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::Divide => try_div_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::Modulus => try_mod_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::BitwiseAnd => try_bitwise_and_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::BitwiseOr => try_bitwise_or_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::BitwiseXor => try_bitwise_xor_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::ShiftLeft => try_shift_left_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::ShiftRight => try_shift_right_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,

                IROperation::And => {
                    let result =
                        try_and_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(result))
                }
                IROperation::Or => {
                    let result =
                        try_or_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
                    gc_system.new_object(VMBoolean::new(result))
                }
                IROperation::Power => try_power_as_vmobject(&left, &right, gc_system)
                    .map_err(VMError::VMVariableError)?,
                _ => {
                    return Err(VMError::DetailedError(
                        "Binary operation not supported".to_string(),
                    ))
                }
            };
        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(())
    }

    pub fn unary_op(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        operation: IROperation,
    ) -> Result<(), VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;
        let obj = match operation {
            IROperation::Not => {
                let result = try_not_as_vmobject(&ref_obj).map_err(VMError::VMVariableError)?;
                gc_system.new_object(VMBoolean::new(result))
            }
            IROperation::Subtract => {
                if ref_obj.isinstance::<VMInt>() {
                    let value = ref_obj.as_const_type::<VMInt>().value;
                    gc_system.new_object(VMInt::new(-value))
                } else if ref_obj.isinstance::<VMFloat>() {
                    let value = ref_obj.as_const_type::<VMFloat>().value;
                    gc_system.new_object(VMFloat::new(-value))
                } else {
                    return Err(VMError::DetailedError(
                        "Unary minus operation not supported".to_string(),
                    ));
                }
            }
            IROperation::Add => {
                if ref_obj.isinstance::<VMInt>() {
                    let value = ref_obj.as_const_type::<VMInt>().value;
                    gc_system.new_object(VMInt::new(value.abs()))
                } else if ref_obj.isinstance::<VMFloat>() {
                    let value = ref_obj.as_const_type::<VMFloat>().value;
                    gc_system.new_object(VMFloat::new(value.abs()))
                } else {
                    return Err(VMError::DetailedError(
                        "Unary plus operation not supported".to_string(),
                    ));
                }
            }
            IROperation::BitwiseNot => try_bitwise_not_as_vmobject(&ref_obj, gc_system)
                .map_err(VMError::VMVariableError)?,
            _ => {
                return Err(VMError::DetailedError(
                    "Unary operation not supported".to_string(),
                ))
            }
        };
        vm.push_vmobject(obj)?;
        ref_obj.drop_ref();

        Ok(())
    }
    pub fn let_var(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        name: String,
    ) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = vm.context.let_var(name, obj, gc_system);
        if result.is_err() {
            return Err(VMError::ContextError(result.unwrap_err()));
        }
        vm.push_vmobject(obj.clone())?;
        Ok(())
    }

    pub fn get_var(
        vm: &mut IRExecutor,
        _gc_system: &mut GCSystem,
        name: String,
    ) -> Result<(), VMError> {
        let obj = vm.context.get_var(&name).map_err(VMError::ContextError)?;
        vm.push_vmobject(obj)?;
        Ok(())
    }

    pub fn set_var(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let reference = &mut vm.pop_object_and_check()?;
        let result = try_assign_as_vmobject(reference, value).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        value.drop_ref();
        reference.drop_ref();
        Ok(())
    }

    pub fn return_value(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        vm.context
            .pop_frame_until_function(&mut vm.stack, &mut vm.lambda_instructions)
            .map_err(VMError::ContextError)?;
        let obj = &mut vm.pop_object_and_check()?;
        let ip_info = vm.stack.pop().unwrap();
        let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info else {
            return Err(VMError::EmptyStack);
        };
        vm.ip = ip as isize;
        let lambda_obj: &mut VMLambda = self_lambda.as_type::<VMLambda>();
        lambda_obj.set_result(obj);

        vm.push_vmobject(obj.clone_ref())?;

        obj.drop_ref();
        self_lambda.drop_ref();

        if use_new_instructions {
            let poped = vm.lambda_instructions.pop();
            poped.unwrap().drop_ref();
        }
        Ok(())
    }
    pub fn raise(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        vm.context
            .pop_frame_until_boundary(&mut vm.stack, &mut vm.lambda_instructions)
            .map_err(VMError::ContextError)?;
        let obj = &mut vm.pop_object_and_check()?;
        let ip_info = vm.stack.pop().unwrap();
        let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info else {
            return Err(VMError::EmptyStack);
        };
        vm.ip = ip as isize;

        vm.push_vmobject(obj.clone_ref())?;

        obj.drop_ref();
        self_lambda.drop_ref();

        if use_new_instructions {
            let poped = vm.lambda_instructions.pop();
            poped.unwrap().drop_ref();
        }
        Ok(())
    }

    pub fn emit(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        if vm.stack.len() < *vm.context.stack_pointers.last().unwrap() {
            return Err(VMError::EmptyStack);
        }
        let obj = &mut vm.pop_object_and_check()?;
        vm.entry_lambda.as_type::<VMLambda>().set_result(obj);
        vm.push_vmobject(obj.clone_ref())?;
        obj.drop_ref();
        Ok(())
    }
    pub fn is_finished(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        if vm.stack.len() < *vm.context.stack_pointers.last().unwrap() {
            return Err(VMError::EmptyStack);
        }
        let mut obj = vm.pop_object_and_check()?;
        if !obj.isinstance::<VMLambda>() {
            return Err(VMError::InvalidArgument(
                obj,
                "Await: Not a lambda".to_string(),
            ));
        }
        let lambda = obj.as_const_type::<VMLambda>();
        let is_finished = lambda.coroutine_status == VMCoroutineStatus::Finished;
        vm.push_vmobject(gc_system.new_object(VMBoolean::new(is_finished)))?;
        obj.drop_ref();
        Ok(())
    }
    pub fn new_frame(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        vm.context
            .new_frame(&mut vm.stack, ContextFrameType::NormalFrame, 0, false);
        Ok(())
    }
    pub fn new_boundary_frame(
        vm: &mut IRExecutor,
        _gc_system: &mut GCSystem,
        offset: isize,
    ) -> Result<(), VMError> {
        vm.stack.push(VMStackObject::LastIP(
            vm.entry_lambda.clone_ref(),
            (vm.ip + offset) as usize, // raise和pop跳转位置
            false,
        ));
        vm.context
            .new_frame(&mut vm.stack, ContextFrameType::BoundaryFrame, 0, false);
        Ok(())
    }
    pub fn pop_frame(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        vm.context
            .pop_frame_except_top(&mut vm.stack, &mut vm.lambda_instructions)
            .map_err(VMError::ContextError)?;
        Ok(())
    }
    pub fn pop_boundary_frame(
        vm: &mut IRExecutor,
        _gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        vm.context
            .pop_frame_except_top(&mut vm.stack, &mut vm.lambda_instructions)
            .map_err(VMError::ContextError)?;
        let obj = vm.pop_object_and_check()?;
        let ip_info = vm.stack.pop().unwrap();
        if let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info {
            vm.ip = ip as isize;
            if use_new_instructions {
                let poped = vm.lambda_instructions.pop();
                poped.unwrap().drop_ref();
            }
            self_lambda.drop_ref();
            vm.push_vmobject(obj)?;
        } else {
            return Err(VMError::DetailedError(
                "PopBoundaryFrame: Not a LastIP".to_string(),
            ));
        };
        Ok(())
    }
    pub fn discard_top(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = vm.pop_object()?;
        let mut obj = match obj {
            VMStackObject::VMObject(obj) => obj,
            _ => return Err(VMError::NotVMObject(obj)),
        };
        obj.drop_ref();
        Ok(())
    }
    pub fn jump_if_false(
        vm: &mut IRExecutor,
        _gc_system: &mut GCSystem,
        offset: isize,
    ) -> Result<(), VMError> {
        let mut obj = vm.pop_object_and_check()?;
        if !obj.isinstance::<VMBoolean>() {
            return Err(VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "JumpIfFalseOffset: Not a boolean".to_string(),
            )));
        }
        if !obj.as_const_type::<VMBoolean>().value {
            vm.ip += offset;
        }
        obj.drop_ref();
        Ok(())
    }
    pub fn reset_stack(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        for i in *vm.context.stack_pointers.last().unwrap()..vm.stack.len() {
            let obj = vm.stack[i].clone();
            if let VMStackObject::VMObject(mut obj) = obj {
                obj.drop_ref();
            }
        }
        vm.stack
            .truncate(*vm.context.stack_pointers.last().unwrap());
        Ok(())
    }
    pub fn get_attr(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut attr = vm.pop_object_and_check()?;
        let obj = &mut vm.pop_object_and_check()?;

        let result = try_get_attr_as_vmobject(obj, &attr).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        attr.drop_ref();
        Ok(())
    }
    pub fn index_of(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut index = vm.pop_object_and_check()?;
        let obj = &mut vm.pop_object_and_check()?;
        let result =
            try_index_of_as_vmobject(obj, &index, gc_system).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result)?; // 不clone是因为已经在try_index_of_as_vmobject产生了新的对象
        obj.drop_ref();
        index.drop_ref();
        Ok(())
    }
    pub fn key_of(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_key_of_as_vmobject(obj).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        Ok(())
    }
    pub fn value_of(vm: &mut IRExecutor, _gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_value_of_as_vmobject(obj).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        Ok(())
    }
    pub fn assert(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut obj = vm.pop_object_and_check()?;
        if !obj.isinstance::<VMBoolean>() {
            return Err(VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Assert: Not a boolean".to_string(),
            )));
        }
        if !obj.as_const_type::<VMBoolean>().value {
            return Err(VMError::AssertFailed);
        }
        obj.drop_ref();
        vm.push_vmobject(gc_system.new_object(VMBoolean::new(true)))?;
        Ok(())
    }
    pub fn self_of(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut obj = vm.pop_object_and_check()?;

        if !obj.isinstance::<VMLambda>() {
            return Err(VMError::CannotGetSelf(obj));
        }
        let lambda = obj.as_type::<VMLambda>();
        let self_obj = lambda.self_object.as_mut();
        match self_obj {
            Some(self_obj) => {
                vm.push_vmobject(self_obj.clone_ref())?;
            }
            None => {
                vm.stack
                    .push(VMStackObject::VMObject(gc_system.new_object(VMNull::new())));
            }
        }
        obj.drop_ref();
        Ok(())
    }
    pub fn deepcopy(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_deepcopy_as_vmobject(obj, gc_system).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Not a copyable object".to_string(),
            ))
        })?;
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(())
    }
    pub fn copy(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_copy_as_vmobject(obj, gc_system).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Not a copyable object".to_string(),
            ))
        })?;
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(())
    }
    pub fn call_lambda(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let arg_tuple = &mut vm.pop_object_and_check()?;
        let lambda = &mut vm.pop_object_and_check()?;

        if lambda.isinstance::<VMNativeFunction>() {
            let lambda_ref = lambda.as_const_type::<VMNativeFunction>();
            let result = lambda_ref.call(arg_tuple.clone(), gc_system);
            if result.is_err() {
                return Err(VMError::VMVariableError(result.unwrap_err()));
            }
            let result = result.unwrap();
            vm.push_vmobject(result)?;
            arg_tuple.drop_ref();
            lambda.drop_ref();
            return Ok(());
        }

        if !lambda.isinstance::<VMLambda>() {
            return Err(VMError::TryEnterNotLambda(lambda.clone()));
        }

        let lambda_obj = lambda.as_type::<VMLambda>();

        let signature = lambda_obj.signature.clone();
        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .assign_members(arg_tuple);
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }
        vm.enter_lambda(lambda, gc_system)?;

        let func_ips = &vm
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .func_ips;
        let ip = *func_ips.get(&signature).unwrap() as isize;
        vm.ip = ip - 1;

        arg_tuple.drop_ref();
        lambda.drop_ref();
        Ok(())
    }
    pub fn async_call_lambda(
        vm: &mut IRExecutor,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let arg_tuple = &mut vm.pop_object_and_check()?;
        let lambda = &mut vm.pop_object_and_check()?;

        if lambda.isinstance::<VMNativeFunction>() {
            return Err(VMError::InvalidArgument(
                lambda.clone(),
                "Native Function doesn't support async".to_string(),
            ));
        }

        if !lambda.isinstance::<VMLambda>() {
            return Err(VMError::TryEnterNotLambda(lambda.clone()));
        }

        let lambda_obj = lambda.as_type::<VMLambda>();

        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .assign_members(arg_tuple);
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }

        let spawned_coroutines = vec![SpawnedCoroutine {
            lambda_ref: lambda.clone_ref(),
            source_code: vm.original_code.clone(),
        }];
        vm.push_vmobject(lambda.clone_ref())?;

        arg_tuple.drop_ref();
        lambda.drop_ref();

        Ok(Some(spawned_coroutines))
    }
    pub fn wrap(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let wrapped = VMWrapper::new(obj);
        let wrapped = gc_system.new_object(wrapped);
        vm.push_vmobject(wrapped)?;
        obj.drop_ref();
        Ok(())
    }

    pub fn is_in(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut container = vm.pop_object_and_check()?;
        let mut obj = vm.pop_object_and_check()?;
        let result = try_contains_as_vmobject(&container, &obj).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                container.clone(),
                "Not a container".to_string(),
            ))
        })?;
        vm.push_vmobject(gc_system.new_object(VMBoolean::new(result)))?;
        container.drop_ref();
        obj.drop_ref();
        Ok(())
    }

    pub fn build_range(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut end = vm.pop_object_and_check()?;
        let mut start = vm.pop_object_and_check()?;

        if !start.isinstance::<VMInt>() {
            return Err(VMError::InvalidArgument(
                start.clone(),
                "Start of range is not a VMInt".to_string(),
            ));
        }
        if !end.isinstance::<VMInt>() {
            return Err(VMError::InvalidArgument(
                end.clone(),
                "End of range is not a VMInt".to_string(),
            ));
        }
        let start_ref = start.as_const_type::<VMInt>();
        let end_ref = end.as_const_type::<VMInt>();

        let result = gc_system.new_object(VMRange::new(start_ref.value, end_ref.value));
        vm.push_vmobject(result)?;
        start.drop_ref();
        end.drop_ref();
        Ok(())
    }
    pub fn type_of(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;

        if ref_obj.isinstance::<VMInt>() {
            let result = gc_system.new_object(VMString::new("int".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMFloat>() {
            let result = gc_system.new_object(VMString::new("float".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMString>() {
            let result = gc_system.new_object(VMString::new("string".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBoolean>() {
            let result = gc_system.new_object(VMString::new("bool".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMTuple>() {
            let result = gc_system.new_object(VMString::new("tuple".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMLambda>() {
            let result = gc_system.new_object(VMString::new("lambda".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMNull>() {
            let result = gc_system.new_object(VMString::new("null".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMKeyVal>() {
            let result = gc_system.new_object(VMString::new("keyval".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMNamed>() {
            let result = gc_system.new_object(VMString::new("named".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMRange>() {
            let result = gc_system.new_object(VMString::new("range".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMWrapper>() {
            let result = gc_system.new_object(VMString::new("wrapper".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBoolean>() {
            let result = gc_system.new_object(VMString::new("bool".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMInstructions>() {
            let result = gc_system.new_object(VMString::new("instructions".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMNativeFunction>() {
            let result = gc_system.new_object(VMString::new("native_function".to_string()));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBytes>() {
            let result = gc_system.new_object(VMString::new("bytes".to_string()));
            vm.push_vmobject(result)?;
        } else {
            let result = gc_system.new_object(VMString::new("".to_string()));
            vm.push_vmobject(result)?;
        }
        ref_obj.drop_ref();
        Ok(())
    }
    pub fn import(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut path_arg = vm.pop_object_and_check()?;

        if !path_arg.isinstance::<VMString>() {
            return Err(VMError::InvalidArgument(
                path_arg.clone(),
                format!(
                    "Import requires VMString but got {}",
                    try_repr_vmobject(path_arg.clone(), None).unwrap_or(format!("{:?}", path_arg))
                ),
            ));
        }

        let path_ref = path_arg.as_const_type::<VMString>();

        let path = path_ref.value.clone();
        let path = path.as_str();
        let file = File::open(path);
        if file.is_err() {
            return Err(VMError::FileError(format!(
                "Cannot open file: {} : {:?}",
                path,
                file.unwrap_err()
            )));
        }
        let mut file = file.unwrap();
        let mut contents = vec![];
        let result = file.read_to_end(&mut contents);
        if result.is_err() {
            return Err(VMError::FileError(format!(
                "Cannot read file: {} : {:?}",
                path,
                result.unwrap_err()
            )));
        }
        let ir_package = bincode::deserialize(&contents);
        if ir_package.is_err() {
            return Err(VMError::FileError(format!(
                "Cannot deserialize file: {} : {:?}",
                path,
                ir_package.unwrap_err()
            )));
        }
        let IRPackage {
            instructions,
            function_ips,
        } = ir_package.unwrap();

        let vm_instructions = gc_system.new_object(VMInstructions::new(
            instructions.clone(),
            function_ips.clone(),
        ));

        vm.push_vmobject(vm_instructions)?;
        path_arg.drop_ref();
        Ok(())
    }
    pub fn alias(
        vm: &mut IRExecutor,
        gc_system: &mut GCSystem,
        alias: String,
    ) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let mut copied = try_copy_as_vmobject(obj, gc_system).map_err(VMError::VMVariableError)?;
        let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
        obj_alias.push(alias);
        vm.push_vmobject(copied)?;
        obj.drop_ref();
        Ok(())
    }
    pub fn wipe_alias(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let mut copied = try_copy_as_vmobject(obj, gc_system).map_err(VMError::VMVariableError)?;
        let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
        obj_alias.clear();
        vm.push_vmobject(copied)?;
        obj.drop_ref();
        Ok(())
    }
    pub fn alias_of(vm: &mut IRExecutor, gc_system: &mut GCSystem) -> Result<(), VMError> {
        let mut obj = vm.pop_object_and_check()?;
        let obj_alias = try_const_alias_as_vmobject(&obj).map_err(VMError::VMVariableError)?;
        let mut tuple = Vec::new();
        for alias in obj_alias.iter() {
            tuple.push(gc_system.new_object(VMString::new(alias.clone())));
        }
        let mut tuple_refs = tuple.iter_mut().collect();
        let result = gc_system.new_object(VMTuple::new(&mut tuple_refs));
        for alias in tuple.iter_mut() {
            alias.drop_ref();
        }
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(())
    }
}

impl IRExecutor {
    pub fn enter_lambda(
        &mut self,
        lambda_object: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        if !lambda_object.isinstance::<VMLambda>() {
            return Err(VMError::TryEnterNotLambda(lambda_object.clone()));
        }

        let use_new_instructions = self.lambda_instructions.is_empty()
            || lambda_object
                .as_const_type::<VMLambda>()
                .lambda_instructions
                != *self.lambda_instructions.last().unwrap();

        self.stack.push(VMStackObject::LastIP(
            lambda_object.clone_ref(),
            self.ip as usize,
            use_new_instructions,
        ));

        let lambda = lambda_object.as_type::<VMLambda>();
        let code_position = lambda.code_position;

        if use_new_instructions {
            self.lambda_instructions
                .push(lambda.lambda_instructions.clone_ref());
        }

        self.context.new_frame(
            &self.stack,
            ContextFrameType::FunctionFrame,
            code_position,
            false,
        );
        let default_args = &mut lambda.default_args_tuple;

        for v_ref in default_args.as_type::<VMTuple>().values.iter_mut() {
            if !v_ref.isinstance::<VMNamed>() {
                return Err(VMError::InvalidArgument(
                    v_ref.clone(),
                    format!(
                        "Not a VMNamed in Lambda arguments: {}",
                        try_repr_vmobject(default_args.clone(), None)
                            .unwrap_or(format!("{:?}", default_args))
                    ),
                ));
            }
            let v = v_ref.as_type::<VMNamed>();
            let name = v.key.clone();
            let value = &mut v.value;

            if !name.isinstance::<VMString>() {
                return Err(VMError::InvalidArgument(
                    name.clone(),
                    format!(
                        "Expected VMString in Lambda arguments {}'s key, but got {}",
                        try_repr_vmobject(v_ref.clone(), None).unwrap_or(format!("{:?}", v_ref)),
                        try_repr_vmobject(name.clone(), None).unwrap_or(format!("{:?}", name))
                    ),
                ));
            }
            let name = name.as_const_type::<VMString>();
            let name = name.value.clone();
            let result = self.context.let_var(name.clone(), value, gc_system);

            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

        if lambda.self_object.is_some() {
            let self_obj_ref = lambda.self_object.as_mut().unwrap();
            let result = self
                .context
                .let_var("self".to_string(), self_obj_ref, gc_system);
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

        Ok(())
    }

    pub fn init(
        &mut self,
        lambda_object: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        self.enter_lambda(lambda_object, gc_system)?;

        self.ip = *self
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .func_ips
            .get(&lambda_object.as_const_type::<VMLambda>().signature)
            .unwrap() as isize;

        //create builtin functions
        IRExecutor::inject_builtin_functions(&mut self.context, gc_system)?;
        Ok(())
    }

    pub fn step(
        &mut self,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        // spawned new coroutine, error

        let coroutine_status = &self
            .entry_lambda
            .as_const_type::<VMLambda>()
            .coroutine_status;

        let mut spawned_coroutines = None;
        if !self.lambda_instructions.is_empty()
            && self.ip
                < self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .instructions
                    .len() as isize
            && *coroutine_status != VMCoroutineStatus::Finished
        {
            let instruction = self
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .instructions[self.ip as usize]
                .clone();

            // if let IR::DebugInfo(_) = instruction {} else{
            //     println!("{}: {:?}", self.ip, instruction); // debug
            // }

            // println!("");
            // if self.debug_info.is_some() {
            //     let debug_info = self.debug_info.as_ref().unwrap();
            //     println!(
            //         "{}: {}",
            //         self.ip,
            //         self.repr_current_code(Some(debug_info.code_position))
            //     ); // debug
            // }
            // self.context.debug_print_all_vars();
            // gc_system.collect(); // debug
            // self.debug_output_stack();
            spawned_coroutines = self.execute_instruction(instruction, gc_system)?;

            //self.debug_output_stack(); // debug
            //println!("");

            //gc_system.collect(); // debug
            //println!("GC Count: {}", gc_system.count()); // debug
            //gc_system.print_reference_graph(); // debug
            self.ip += 1;
        } else if *coroutine_status != VMCoroutineStatus::Finished {
            let mut result = self.pop_object_and_check()?;
            let lambda_obj = self.entry_lambda.as_type::<VMLambda>();
            lambda_obj.coroutine_status = VMCoroutineStatus::Finished;
            lambda_obj.set_result(&mut result);
            result.drop_ref();
        }

        Ok(spawned_coroutines)
    }
}

impl IRExecutor {
    fn push_vmobject(&mut self, obj: GCRef) -> Result<(), VMError> {
        self.stack.push(VMStackObject::VMObject(obj.clone()));
        Ok(())
    }

    pub fn execute_instruction(
        &mut self,
        instruction: IR,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        match &instruction {
            IR::LoadInt(value) => {
                vm_instructions::load_int(self, gc_system, *value)?;
            }
            IR::LoadFloat(value) => {
                vm_instructions::load_float(self, gc_system, *value)?;
            }
            IR::LoadString(value) => {
                vm_instructions::load_string(self, gc_system, value.clone())?;
            }
            IR::LoadBool(value) => {
                vm_instructions::load_bool(self, gc_system, *value)?;
            }
            IR::LoadNull => {
                vm_instructions::load_null(self, gc_system)?;
            }
            IR::LoadBytes(value) => {
                vm_instructions::load_bytes(self, gc_system, value.clone())?;
            }
            IR::LoadLambda(signature, code_position) => {
                vm_instructions::load_lambda(self, gc_system, signature.clone(), *code_position)?;
            }
            IR::ForkInstruction => {
                vm_instructions::fork_instruction(self, gc_system)?;
            }
            IR::BuildTuple(size) => {
                vm_instructions::build_tuple(self, gc_system, *size)?;
            }
            IR::BindSelf => {
                vm_instructions::bind_self(self, gc_system)?;
            }
            IR::BuildKeyValue => {
                vm_instructions::build_keyval(self, gc_system)?;
            }
            IR::BuildNamed => {
                vm_instructions::build_named(self, gc_system)?;
            }
            IR::BinaryOp(operation) => {
                vm_instructions::binary_op(self, gc_system, operation.clone())?;
            }
            IR::UnaryOp(operation) => {
                vm_instructions::unary_op(self, gc_system, operation.clone())?;
            }
            IR::Let(name) => {
                vm_instructions::let_var(self, gc_system, name.clone())?;
            }
            IR::Get(name) => {
                vm_instructions::get_var(self, gc_system, name.clone())?;
            }
            IR::Set => {
                vm_instructions::set_var(self, gc_system)?;
            }
            IR::Return => {
                vm_instructions::return_value(self, gc_system)?;
            }
            IR::Raise => {
                vm_instructions::raise(self, gc_system)?;
            }
            IR::Emit => {
                vm_instructions::emit(self, gc_system)?;
            }
            IR::IsFinished => {
                vm_instructions::is_finished(self, gc_system)?;
            }
            IR::NewFrame => {
                vm_instructions::new_frame(self, gc_system)?;
            }
            IR::NewBoundaryFrame(offset) => {
                vm_instructions::new_boundary_frame(self, gc_system, *offset)?;
            }
            IR::PopFrame => {
                vm_instructions::pop_frame(self, gc_system)?;
            }
            IR::PopBoundaryFrame => {
                vm_instructions::pop_boundary_frame(self, gc_system)?;
            }
            IR::Pop => {
                vm_instructions::discard_top(self, gc_system)?;
            }
            IR::JumpOffset(offset) => {
                self.ip += offset;
            }
            IR::JumpIfFalseOffset(offset) => {
                vm_instructions::jump_if_false(self, gc_system, *offset)?;
            }
            IR::ResetStack => {
                vm_instructions::reset_stack(self, gc_system)?;
            }
            IR::GetAttr => {
                vm_instructions::get_attr(self, gc_system)?;
            }
            IR::IndexOf => {
                vm_instructions::index_of(self, gc_system)?;
            }
            IR::KeyOf => {
                vm_instructions::key_of(self, gc_system)?;
            }
            IR::ValueOf => {
                vm_instructions::value_of(self, gc_system)?;
            }
            IR::Assert => {
                vm_instructions::assert(self, gc_system)?;
            }
            IR::SelfOf => {
                vm_instructions::self_of(self, gc_system)?;
            }
            IR::DeepCopyValue => {
                vm_instructions::deepcopy(self, gc_system)?;
            }
            IR::CopyValue => {
                vm_instructions::copy(self, gc_system)?;
            }
            IR::DebugInfo(debug_info) => {
                if self.original_code.is_some() {
                    let debug_info = debug_info.clone();
                    self.set_debug_info(debug_info);
                }
            }
            IR::CallLambda => {
                vm_instructions::call_lambda(self, gc_system)?;
            }
            IR::AsyncCallLambda => {
                return vm_instructions::async_call_lambda(self, gc_system);
            }
            IR::Wrap => {
                vm_instructions::wrap(self, gc_system)?;
            }
            IR::In => {
                vm_instructions::is_in(self, gc_system)?;
            }
            IR::BuildRange => {
                vm_instructions::build_range(self, gc_system)?;
            }
            IR::TypeOf => {
                vm_instructions::type_of(self, gc_system)?;
            }
            IR::Import => {
                vm_instructions::import(self, gc_system)?;
            }
            IR::Alias(alias) => {
                vm_instructions::alias(self, gc_system, alias.clone())?;
            }
            IR::WipeAlias => {
                vm_instructions::wipe_alias(self, gc_system)?;
            }
            IR::AliasOf => {
                vm_instructions::alias_of(self, gc_system)?;
            }
            _ => return Err(VMError::InvalidInstruction(instruction.clone())),
        }

        Ok(None)
    }
}
