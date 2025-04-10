use crate::vm::instruction_set::VMInstruction;
use crate::vm::opcode::Instruction32;
use crate::vm::opcode::ProcessedOpcode;

use super::super::gc::gc::*;
use super::context::*;
use super::native_function::native_functions;
use super::variable::*;
use super::vm_instructions::vm_instructions;

#[derive(Debug)]
pub enum VMError {
    InvalidInstruction(ProcessedOpcode),
    TryEnterNotLambda(GCRef),
    EmptyStack,
    ArgumentIsNotTuple(GCRef),
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
    executors: Vec<(VMExecutor, isize)>, // executor, id
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
        gc_system: &mut GCSystem,
    ) -> Result<isize, VMError> {
        if !lambda_object.isinstance::<VMLambda>() {
            return Err(VMError::DetailedError(
                "lambda_object must be a VMLambda".to_string(),
            ));
        }
        let mut executor = VMExecutor::new(&lambda_object.clone_ref());

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
                    self.new_coroutine(lambda_object, gc_system)?;
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
    pub(super) lambda_ref: GCRef,
}

type InstructionHandler = fn(
    &mut VMExecutor,
    &ProcessedOpcode,
    &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError>;

#[derive(Debug)]
pub struct VMExecutor {
    pub(super) context: Context,
    pub(super) lambda_instructions: Vec<GCRef>,
    pub(super) stack: Vec<VMStackObject>,
    pub(super) ip: isize,
    pub(super) entry_lambda: GCRef,
    pub(super) instruction_table: Vec<InstructionHandler>,
}

impl VMExecutor {
    pub fn new(entry_lambda: &GCRef) -> Self {
        let mut instruction_table: Vec<InstructionHandler> = vec![
        |_, opcode, _| Err(VMError::InvalidInstruction(opcode.clone())); // 默认处理函数 - 返回无效指令错误
        256 // 数组大小，确保能容纳所有可能的操作码
    ];

        // 使用枚举值作为索引，填充对应的处理函数
        // 栈操作
        instruction_table[VMInstruction::LoadNull as usize] = vm_instructions::load_null;
        instruction_table[VMInstruction::LoadInt32 as usize] = vm_instructions::load_int;
        instruction_table[VMInstruction::LoadInt64 as usize] = vm_instructions::load_int;
        instruction_table[VMInstruction::LoadFloat32 as usize] = vm_instructions::load_float;
        instruction_table[VMInstruction::LoadFloat64 as usize] = vm_instructions::load_float;
        instruction_table[VMInstruction::LoadString as usize] = vm_instructions::load_string;
        instruction_table[VMInstruction::LoadBytes as usize] = vm_instructions::load_bytes;
        instruction_table[VMInstruction::LoadBool as usize] = vm_instructions::load_bool;
        instruction_table[VMInstruction::LoadLambda as usize] = vm_instructions::load_lambda;
        instruction_table[VMInstruction::Pop as usize] = vm_instructions::discard_top;

        // 数据结构构建
        instruction_table[VMInstruction::BuildTuple as usize] = vm_instructions::build_tuple;
        instruction_table[VMInstruction::BuildKeyValue as usize] = vm_instructions::build_keyval;
        instruction_table[VMInstruction::BuildNamed as usize] = vm_instructions::build_named;
        instruction_table[VMInstruction::BuildRange as usize] = vm_instructions::build_range;
        instruction_table[VMInstruction::BuildSet as usize] = vm_instructions::build_set;
        // 二元操作符
        instruction_table[VMInstruction::BinaryAdd as usize] = vm_instructions::binary_add;
        instruction_table[VMInstruction::BinarySub as usize] = vm_instructions::binary_subtract;
        instruction_table[VMInstruction::BinaryMul as usize] = vm_instructions::binary_multiply;
        instruction_table[VMInstruction::BinaryDiv as usize] = vm_instructions::binary_divide;
        instruction_table[VMInstruction::BinaryMod as usize] = vm_instructions::binary_modulus;
        instruction_table[VMInstruction::BinaryPow as usize] = vm_instructions::binary_power;
        instruction_table[VMInstruction::BinaryBitAnd as usize] =
            vm_instructions::binary_bitwise_and;
        instruction_table[VMInstruction::BinaryBitOr as usize] = vm_instructions::binary_bitwise_or;
        instruction_table[VMInstruction::BinaryBitXor as usize] =
            vm_instructions::binary_bitwise_xor;
        instruction_table[VMInstruction::BinaryShl as usize] = vm_instructions::binary_shift_left;
        instruction_table[VMInstruction::BinaryShr as usize] = vm_instructions::binary_shift_right;
        instruction_table[VMInstruction::BinaryEq as usize] = vm_instructions::binary_equal;
        instruction_table[VMInstruction::BinaryNe as usize] = vm_instructions::binary_not_equal;
        instruction_table[VMInstruction::BinaryGt as usize] = vm_instructions::binary_greater;
        instruction_table[VMInstruction::BinaryLt as usize] = vm_instructions::binary_less;
        instruction_table[VMInstruction::BinaryGe as usize] = vm_instructions::binary_greater_equal;
        instruction_table[VMInstruction::BinaryLe as usize] = vm_instructions::binary_less_equal;
        instruction_table[VMInstruction::BinaryIn as usize] = vm_instructions::is_in;

        // 一元操作
        instruction_table[VMInstruction::UnaryBitNot as usize] = vm_instructions::unary_bitwise_not;
        instruction_table[VMInstruction::UnaryAbs as usize] = vm_instructions::unary_plus;
        instruction_table[VMInstruction::UnaryNeg as usize] = vm_instructions::unary_minus;

        // 变量与引用
        instruction_table[VMInstruction::StoreVar as usize] = vm_instructions::let_var;
        instruction_table[VMInstruction::LoadVar as usize] = vm_instructions::get_var;
        instruction_table[VMInstruction::SetValue as usize] = vm_instructions::set_var;
        instruction_table[VMInstruction::WrapObj as usize] = vm_instructions::wrap;
        instruction_table[VMInstruction::GetAttr as usize] = vm_instructions::get_attr;
        instruction_table[VMInstruction::IndexOf as usize] = vm_instructions::index_of;
        instruction_table[VMInstruction::KeyOf as usize] = vm_instructions::key_of;
        instruction_table[VMInstruction::ValueOf as usize] = vm_instructions::value_of;
        instruction_table[VMInstruction::SelfOf as usize] = vm_instructions::self_of;
        instruction_table[VMInstruction::TypeOf as usize] = vm_instructions::type_of;
        instruction_table[VMInstruction::DeepCopy as usize] = vm_instructions::deepcopy;
        instruction_table[VMInstruction::ShallowCopy as usize] = vm_instructions::copy;
        instruction_table[VMInstruction::Swap as usize] = vm_instructions::swap;
        instruction_table[VMInstruction::ForkStackObjectRef as usize] =
            vm_instructions::fork_stack_object_ref;
        instruction_table[VMInstruction::PushValueIntoTuple as usize] =
            vm_instructions::push_value_into_tuple;
        instruction_table[VMInstruction::ResetIter as usize] = vm_instructions::reset_iter;
        instruction_table[VMInstruction::NextOrJump as usize] = vm_instructions::next_or_jump;

        // 控制流
        instruction_table[VMInstruction::Call as usize] = vm_instructions::call_lambda;
        instruction_table[VMInstruction::AsyncCall as usize] = vm_instructions::async_call_lambda;
        instruction_table[VMInstruction::Return as usize] = vm_instructions::return_value;
        instruction_table[VMInstruction::Raise as usize] = vm_instructions::raise;
        instruction_table[VMInstruction::Jump as usize] = vm_instructions::jump;
        instruction_table[VMInstruction::JumpIfFalse as usize] = vm_instructions::jump_if_false;

        // 帧操作
        instruction_table[VMInstruction::NewFrame as usize] = vm_instructions::new_frame;
        instruction_table[VMInstruction::NewBoundaryFrame as usize] =
            vm_instructions::new_boundary_frame;
        instruction_table[VMInstruction::PopFrame as usize] = vm_instructions::pop_frame;
        instruction_table[VMInstruction::PopBoundaryFrame as usize] =
            vm_instructions::pop_boundary_frame;
        instruction_table[VMInstruction::ResetStack as usize] = vm_instructions::reset_stack;

        // 模块操作
        instruction_table[VMInstruction::Import as usize] = vm_instructions::import;

        // 特殊操作
        instruction_table[VMInstruction::Fork as usize] = vm_instructions::fork_instruction;
        instruction_table[VMInstruction::BindSelf as usize] = vm_instructions::bind_self;
        instruction_table[VMInstruction::Assert as usize] = vm_instructions::assert;
        instruction_table[VMInstruction::Emit as usize] = vm_instructions::emit;
        instruction_table[VMInstruction::IsFinished as usize] = vm_instructions::is_finished;

        // 别名操作
        instruction_table[VMInstruction::Alias as usize] = vm_instructions::alias;
        instruction_table[VMInstruction::WipeAlias as usize] = vm_instructions::wipe_alias;
        instruction_table[VMInstruction::AliasOf as usize] = vm_instructions::alias_of;

        VMExecutor {
            context: Context::new(),
            stack: Vec::new(),
            ip: 0,
            lambda_instructions: Vec::new(),
            entry_lambda: entry_lambda.clone(),
            instruction_table,
        }
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

    pub fn get_object_and_check(&self, index: usize) -> Result<GCRef, VMError> {
        if index >= self.stack.len() {
            return Err(VMError::EmptyStack);
        }
        let obj = &self.stack[self.stack.len() - 1 - index];
        if let VMStackObject::VMObject(obj) = obj {
            return Ok(obj.clone());
        }
        Err(VMError::NotVMObject(obj.clone()))
    }

    pub fn push_vmobject(&mut self, obj: GCRef) -> Result<(), VMError> {
        self.stack.push(VMStackObject::VMObject(obj.clone()));
        Ok(())
    }
    pub fn _debug_output_stack(&self) {
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

impl VMExecutor {
    pub fn enter_lambda(
        &mut self,
        lambda_object: &mut GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        if !lambda_object.isinstance::<VMLambda>() {
            return Err(VMError::TryEnterNotLambda(lambda_object.clone()));
        }

        let VMLambdaBody::VMInstruction(lambda_body) =
            &lambda_object.as_const_type::<VMLambda>().lambda_body
        else {
            return Err(VMError::InvalidArgument(
                lambda_object.clone(),
                "Only lambda defined by VMInstruction can be entered".to_string(),
            ));
        };
        let use_new_instructions = self.lambda_instructions.is_empty()
            || *lambda_body != *self.lambda_instructions.last().unwrap();

        self.stack.push(VMStackObject::LastIP(
            lambda_object.clone_ref(),
            self.ip as usize,
            use_new_instructions,
        ));

        let lambda = lambda_object.as_type::<VMLambda>();

        let VMLambdaBody::VMInstruction(lambda_body) = &mut lambda.lambda_body else {
            return Err(VMError::InvalidArgument(
                lambda_object.clone(),
                "Only lambda defined by VMInstruction can be entered".to_string(),
            ));
        };
        let code_position = lambda.code_position;

        if use_new_instructions {
            self.lambda_instructions.push(lambda_body.clone_ref());
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
                // return Err(VMError::InvalidArgument(
                //     v_ref.clone(),
                //     format!(
                //         "Not a VMNamed in Lambda arguments: {}",
                //         try_repr_vmobject(default_args.clone(), None)
                //             .unwrap_or(format!("{:?}", default_args))
                //     ),
                // ));
                continue;
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
            let result = self.context.let_var(&name.value, value, gc_system);

            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

        if lambda.self_object.is_some() {
            let self_obj_ref = lambda.self_object.as_mut().unwrap();
            let result = self.context.let_var("self", self_obj_ref, gc_system);
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

        let result = self.context.let_var("this", lambda_object, gc_system);
        if result.is_err() {
            return Err(VMError::ContextError(result.unwrap_err()));
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
            .vm_instructions_package
            .get_table()
            .get(&lambda_object.as_const_type::<VMLambda>().signature)
            .unwrap() as isize;

        //create builtin functions
        native_functions::inject_builtin_functions(&mut self.context, gc_system)?;
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
        if !self.lambda_instructions.is_empty() && *coroutine_status != VMCoroutineStatus::Finished
        {
            let vm_instruction = self
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>();
            if self.ip < vm_instruction.vm_instructions_package.get_code().len() as isize {
                let mut ip = self.ip as usize;
                let curr_ip = self.ip;
                let code: &Vec<u32> = vm_instruction.vm_instructions_package.get_code();
                let mut instruction_32 = Instruction32::new(code, &mut ip);

                let decoded = instruction_32.get_processed_opcode();
                if decoded.is_none() {
                    return Err(VMError::AssertFailed);
                }
                let decoded = decoded.unwrap();
                self.ip = ip as isize;
                // if let IR::DebugInfo(_) = instruction {} else{
                //     println!("{}: {:?}", self.ip, instruction); // debug
                // }

                // println!("");
                // self.context.debug_print_all_vars();
                // gc_system.collect(); // debug
                // self.debug_output_stack();
                // println!("{}: {}", self.ip, decoded.to_string()); // debug
                let spawned_coroutine = self
                    .instruction_table
                    .get(decoded.instruction as usize)
                    .unwrap()(self, &decoded, gc_system);
                if spawned_coroutine.is_err() {
                    self.ip = curr_ip;
                    return Err(spawned_coroutine.err().unwrap());
                }
                spawned_coroutines = spawned_coroutine.unwrap();

                //self.debug_output_stack(); // debug
                //println!("");

                //gc_system.collect(); // debug
                //println!("GC Count: {}", gc_system.count()); // debug
                //gc_system.print_reference_graph(); // debug
                //self.ip += 1;
            }
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

impl VMExecutor {
    pub fn repr_current_code(&self, context_lines: Option<usize>) -> String {
        use colored::*;
        use unicode_segmentation::UnicodeSegmentation;

        let context_lines = context_lines.unwrap_or(2); // Default to 2 lines of context

        let instruction_package = self
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>();
        let original_code = instruction_package.vm_instructions_package.get_source();

        let debug_info = instruction_package
            .vm_instructions_package
            .get_debug_info()
            .get(&(self.ip as usize));

        if original_code.is_none() || debug_info.is_none() {
            return String::from("[Source code information not available]")
                .bright_yellow()
                .italic()
                .to_string();
        }

        let source_code = original_code.as_ref().unwrap();
        let debug_info = debug_info.unwrap();

        // Check if current IP is out of bounds
        if self.ip < 0
            || self.ip as usize
                >= self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .vm_instructions_package
                    .get_code()
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
                lines[i].bright_white().underline().bold().to_string()
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
                marker.push_str(&"^".bright_red().bold().to_string());

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
            .vm_instructions_package
            .get_code()[self.ip as usize];

        let decoded = Instruction32::decode_opcode(*instruction);

        // Format current instruction info
        result.push_str(&format!(
            "{} {} {}\n",
            "Current instruction:".bright_blue().bold(),
            decoded.to_string().bright_cyan().bold().underline(),
            format!("(IP: {})", self.ip).bright_blue().bold()
        ));

        result
    }
}
