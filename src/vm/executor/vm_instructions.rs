pub mod vm_instructions {
    use crate::vm::executor::context::ContextFrameType;
    use crate::vm::executor::variable::*;
    use crate::vm::executor::vm::VMError;
    use crate::vm::gc::gc::GCSystem;
    use crate::vm::opcode::{OpcodeArgument, ProcessedOpcode};
    use crate::VMExecutor;
    use std::fs::File;
    use std::io::Read;

    use crate::vm::executor::vm::SpawnedCoroutine;

    pub fn load_int(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Int64(value) = opcode.operand1 {
            let obj = gc_system.new_object(VMInt::new(value));
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn load_float(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Float64(value) = opcode.operand1 {
            let obj = gc_system.new_object(VMFloat::new(value));
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn load_string(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::String(value) = opcode.operand1 {
            let str = &vm
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .vm_instructions_package
                .get_string_pool()[value as usize];
            let obj = gc_system.new_object(VMString::new(str));
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn load_bool(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Int32(value) = opcode.operand1 {
            let obj = gc_system.new_object(VMBoolean::new(value != 0));
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn load_null(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = gc_system.new_object(VMNull::new());
        vm.push_vmobject(obj)?;
        Ok(None)
    }
    pub fn load_bytes(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::ByteArray(value) = opcode.operand1 {
            let bytes = &vm
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .vm_instructions_package
                .get_bytes_pool()[value as usize];
            let obj = gc_system.new_object(VMBytes::new(bytes));
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn load_lambda(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::String(sign_idx) = opcode.operand1 {
            if let OpcodeArgument::Int64(code_position) = opcode.operand2 {
                let instruction = &mut vm.pop_object_and_check()?;
                if !instruction.isinstance::<VMInstructions>()
                    && !instruction.isinstance::<VMCLambdaInstruction>()
                {
                    return Err(VMError::InvalidArgument(
                        instruction.clone(),
                        "LoadLambda requires a VMInstructions or VMCLambdaInstruction".to_string(),
                    ));
                }
                let default_args_tuple = &mut vm.pop_object_and_check()?;
                if !default_args_tuple.isinstance::<VMTuple>() {
                    return Err(VMError::ArgumentIsNotTuple(default_args_tuple.clone()));
                }

                let signature = &vm
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .vm_instructions_package
                    .get_string_pool()[sign_idx as usize];

                let mut lambda_result = gc_system.new_object(VMNull::new());
                let obj = gc_system.new_object(VMLambda::new(
                    code_position as usize,
                    signature.clone(),
                    default_args_tuple,
                    None,
                    &mut VMLambdaBody::VMInstruction(instruction.clone()),
                    &mut lambda_result,
                ));
                vm.push_vmobject(obj)?;
                default_args_tuple.drop_ref();
                lambda_result.drop_ref();
                instruction.drop_ref();
                Ok(None)
            } else {
                Err(VMError::InvalidInstruction(opcode.clone()))
            }
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn fork_instruction(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let forked = vm.lambda_instructions.last_mut().unwrap().clone_ref();
        vm.push_vmobject(forked)?;
        Ok(None)
    }
    pub fn build_tuple(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Int64(size) = opcode.operand1 {
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
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }
    pub fn bind_self(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }

    pub fn build_keyval(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let key = &mut vm.pop_object_and_check()?;
        let obj = gc_system.new_object(VMKeyVal::new(key, value));
        vm.push_vmobject(obj)?;
        key.drop_ref();
        value.drop_ref();
        Ok(None)
    }
    pub fn build_named(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let key = &mut vm.pop_object_and_check()?;
        let obj = gc_system.new_object(VMNamed::new(key, value));
        vm.push_vmobject(obj)?;
        key.drop_ref();
        value.drop_ref();
        Ok(None)
    }
    // 将原有的 binary_op 函数拆分为单独的函数
    pub fn binary_add(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = try_add_as_vmobject(&mut left, &mut right, gc_system)
            .map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_subtract(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_sub_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_multiply(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_mul_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_divide(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_div_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_modulus(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_mod_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_power(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_power_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_bitwise_and(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_and_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_bitwise_or(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = try_or_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_bitwise_xor(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj =
            try_xor_as_vmobject(&left, &right, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_shift_left(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = try_shift_left_as_vmobject(&left, &right, gc_system)
            .map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_shift_right(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = try_shift_right_as_vmobject(&left, &right, gc_system)
            .map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_equal(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = gc_system.new_object(VMBoolean::new(try_eq_as_vmobject(&left, &right)));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_not_equal(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let obj = gc_system.new_object(VMBoolean::new(!try_eq_as_vmobject(&left, &right)));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_greater(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let result =
            try_greater_than_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
        let obj = gc_system.new_object(VMBoolean::new(result));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_less(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let result = try_less_than_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
        let obj = gc_system.new_object(VMBoolean::new(result));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_greater_equal(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let result = try_less_than_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
        let obj = gc_system.new_object(VMBoolean::new(!result));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn binary_less_equal(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut right = vm.pop_object_and_check()?;
        let mut left = vm.pop_object_and_check()?;

        let result =
            try_greater_than_as_vmobject(&left, &right).map_err(VMError::VMVariableError)?;
        let obj = gc_system.new_object(VMBoolean::new(!result));

        vm.push_vmobject(obj)?;
        left.drop_ref();
        right.drop_ref();
        Ok(None)
    }

    pub fn unary_minus(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;

        let obj = if ref_obj.isinstance::<VMInt>() {
            let value = ref_obj.as_const_type::<VMInt>().value;
            gc_system.new_object(VMInt::new(-value))
        } else if ref_obj.isinstance::<VMFloat>() {
            let value = ref_obj.as_const_type::<VMFloat>().value;
            gc_system.new_object(VMFloat::new(-value))
        } else {
            return Err(VMError::DetailedError(
                "Unary minus operation not supported".to_string(),
            ));
        };

        vm.push_vmobject(obj)?;
        ref_obj.drop_ref();
        Ok(None)
    }

    pub fn unary_plus(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;

        let obj = if ref_obj.isinstance::<VMInt>() {
            let value = ref_obj.as_const_type::<VMInt>().value;
            gc_system.new_object(VMInt::new(value.abs()))
        } else if ref_obj.isinstance::<VMFloat>() {
            let value = ref_obj.as_const_type::<VMFloat>().value;
            gc_system.new_object(VMFloat::new(value.abs()))
        } else {
            return Err(VMError::DetailedError(
                "Unary plus operation not supported".to_string(),
            ));
        };

        vm.push_vmobject(obj)?;
        ref_obj.drop_ref();
        Ok(None)
    }

    pub fn unary_bitwise_not(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;

        let obj = try_not_as_vmobject(&ref_obj, gc_system).map_err(VMError::VMVariableError)?;

        vm.push_vmobject(obj)?;
        ref_obj.drop_ref();
        Ok(None)
    }
    pub fn let_var(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Int64(name_idx) = opcode.operand1 {
            let obj = &mut vm.pop_object_and_check()?;
            let name = &vm
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .vm_instructions_package
                .get_string_pool()[name_idx as usize];
            let result = vm.context.let_var(name, obj, gc_system);
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
            vm.push_vmobject(obj.clone())?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }

    pub fn get_var(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if let OpcodeArgument::Int64(name_idx) = opcode.operand1 {
            let name = &vm
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .vm_instructions_package
                .get_string_pool()[name_idx as usize];
            let obj = vm.context.get_var(name).map_err(VMError::ContextError)?;
            vm.push_vmobject(obj)?;
            Ok(None)
        } else {
            Err(VMError::InvalidInstruction(opcode.clone()))
        }
    }

    pub fn set_var(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let value = &mut vm.pop_object_and_check()?;
        let reference = &mut vm.pop_object_and_check()?;
        let result = try_assign_as_vmobject(reference, value).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        value.drop_ref();
        reference.drop_ref();
        Ok(None)
    }

    pub fn return_value(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn raise(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }

    pub fn emit(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        if vm.stack.len() < *vm.context.stack_pointers.last().unwrap() {
            return Err(VMError::EmptyStack);
        }
        let obj = &mut vm.pop_object_and_check()?;
        vm.entry_lambda.as_type::<VMLambda>().set_result(obj);
        vm.push_vmobject(obj.clone_ref())?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn is_finished(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn new_frame(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        vm.context
            .new_frame(&mut vm.stack, ContextFrameType::NormalFrame, 0, false);
        Ok(None)
    }
    pub fn new_boundary_frame(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        vm.stack.push(VMStackObject::LastIP(
            vm.entry_lambda.clone_ref(),
            (vm.ip + offset as isize) as usize, // raise和pop跳转位置
            false,
        ));
        vm.context
            .new_frame(&mut vm.stack, ContextFrameType::BoundaryFrame, 0, false);
        Ok(None)
    }
    pub fn pop_frame(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        vm.context
            .pop_frame_except_top(&mut vm.stack, &mut vm.lambda_instructions)
            .map_err(VMError::ContextError)?;
        Ok(None)
    }
    pub fn pop_boundary_frame(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn discard_top(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = vm.pop_object()?;
        let mut obj = match obj {
            VMStackObject::VMObject(obj) => obj,
            _ => return Err(VMError::NotVMObject(obj)),
        };
        obj.drop_ref();
        Ok(None)
    }
    pub fn jump(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        vm.ip += offset as isize;
        Ok(None)
    }
    pub fn jump_if_false(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let mut obj = vm.pop_object_and_check()?;
        if !obj.isinstance::<VMBoolean>() {
            return Err(VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "JumpIfFalseOffset: Not a boolean".to_string(),
            )));
        }
        if !obj.as_const_type::<VMBoolean>().value {
            vm.ip += offset as isize;
        }
        obj.drop_ref();
        Ok(None)
    }
    pub fn reset_stack(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        for i in *vm.context.stack_pointers.last().unwrap()..vm.stack.len() {
            let obj = vm.stack[i].clone();
            if let VMStackObject::VMObject(mut obj) = obj {
                obj.drop_ref();
            }
        }
        vm.stack
            .truncate(*vm.context.stack_pointers.last().unwrap());
        Ok(None)
    }
    pub fn get_attr(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut attr = vm.pop_object_and_check()?;
        let obj = &mut vm.pop_object_and_check()?;

        let result = try_get_attr_as_vmobject(obj, &attr).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        attr.drop_ref();
        Ok(None)
    }
    pub fn index_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut index = vm.pop_object_and_check()?;
        let obj = &mut vm.pop_object_and_check()?;
        let result =
            try_index_of_as_vmobject(obj, &index, gc_system).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result)?; // 不clone是因为已经在try_index_of_as_vmobject产生了新的对象
        obj.drop_ref();
        index.drop_ref();
        Ok(None)
    }
    pub fn key_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_key_of_as_vmobject(obj).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn value_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_value_of_as_vmobject(obj).map_err(VMError::VMVariableError)?;
        vm.push_vmobject(result.clone_ref())?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn assert(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn self_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn deepcopy(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_deepcopy_as_vmobject(obj, gc_system).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Not a copyable object".to_string(),
            ))
        })?;
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn copy(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let result = try_copy_as_vmobject(obj, gc_system).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                obj.clone(),
                "Not a copyable object".to_string(),
            ))
        })?;
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn call_lambda(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let arg_tuple = &mut vm.pop_object_and_check()?;
        let lambda = &mut vm.pop_object_and_check()?;

        // if lambda.isinstance::<VMNativeFunction>() {
        //     let lambda_ref = lambda.as_const_type::<VMNativeFunction>();
        //     let result = lambda_ref.call(arg_tuple.clone(), gc_system);
        //     if result.is_err() {
        //         return Err(VMError::VMVariableError(result.unwrap_err()));
        //     }
        //     let result = result.unwrap();
        //     vm.push_vmobject(result)?;
        //     arg_tuple.drop_ref();
        //     lambda.drop_ref();
        //     return Ok(None);
        // }

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

        let clambda_signature = lambda_obj.alias_const().first().unwrap_or(&lambda_obj.signature).clone();
        match lambda_obj.lambda_body {
            VMLambdaBody::VMInstruction(ref mut body) => {
                if body.isinstance::<VMCLambdaInstruction>() {
                    let clambda = body.as_type::<VMCLambdaInstruction>();
                    let mut result = clambda
                        .call(&clambda_signature, &mut lambda_obj.default_args_tuple, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?;
                    lambda_obj.set_result(&mut result);
                    vm.push_vmobject(result)?;
                    arg_tuple.drop_ref();
                    lambda.drop_ref();
                    return Ok(None);
                }

                vm.enter_lambda(lambda, gc_system)?;

                let func_ips = &vm
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .vm_instructions_package
                    .get_table();
                let ip = *func_ips.get(&signature).unwrap() as isize;
                vm.ip = ip;

                arg_tuple.drop_ref();
                lambda.drop_ref();
                Ok(None)
            }
            VMLambdaBody::VMNativeFunction(native_function) => {
                let result = native_function(arg_tuple.clone(), gc_system);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let mut result = result.unwrap();
                lambda_obj.set_result(&mut result);
                vm.push_vmobject(result)?;
                arg_tuple.drop_ref();
                lambda.drop_ref();
                Ok(None)
            }
        }
    }
    pub fn async_call_lambda(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let arg_tuple = &mut vm.pop_object_and_check()?;
        let lambda = &mut vm.pop_object_and_check()?;

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

        match lambda_obj.lambda_body {
            VMLambdaBody::VMInstruction(ref mut body) => {
                if body.isinstance::<VMCLambdaInstruction>() {
                    return Err(VMError::InvalidArgument(
                        arg_tuple.clone(),
                        "Async call not supported for VMCLambdaInstruction".to_string(),
                    ));
                }
                let spawned_coroutines = vec![SpawnedCoroutine {
                    lambda_ref: lambda.clone_ref(),
                }];
                vm.push_vmobject(lambda.clone_ref())?;

                arg_tuple.drop_ref();
                lambda.drop_ref();

                Ok(Some(spawned_coroutines))
            }
            VMLambdaBody::VMNativeFunction(_) => Err(VMError::InvalidArgument(
                arg_tuple.clone(),
                "Native function cannot be async".to_string(),
            )),
        }
    }
    pub fn wrap(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let wrapped = VMWrapper::new(obj);
        let wrapped = gc_system.new_object(wrapped);
        vm.push_vmobject(wrapped)?;
        obj.drop_ref();
        Ok(None)
    }

    pub fn is_in(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let container = vm.get_object_and_check(0)?;
        let obj = vm.get_object_and_check(1)?;
        let result = try_contains_as_vmobject(&container, &obj).map_err(|_| {
            VMError::VMVariableError(VMVariableError::TypeError(
                container.clone(),
                "Not a container".to_string(),
            ))
        })?;
        vm.push_vmobject(gc_system.new_object(VMBoolean::new(result)))?;
        Ok(None)
    }

    pub fn build_range(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        Ok(None)
    }
    pub fn type_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut ref_obj = vm.pop_object_and_check()?;

        if ref_obj.isinstance::<VMInt>() {
            let result = gc_system.new_object(VMString::new("int"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMFloat>() {
            let result = gc_system.new_object(VMString::new("float"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMString>() {
            let result = gc_system.new_object(VMString::new("string"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBoolean>() {
            let result = gc_system.new_object(VMString::new("bool"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMTuple>() {
            let result = gc_system.new_object(VMString::new("tuple"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMLambda>() {
            let result = gc_system.new_object(VMString::new("lambda"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMNull>() {
            let result = gc_system.new_object(VMString::new("null"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMKeyVal>() {
            let result = gc_system.new_object(VMString::new("keyval"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMNamed>() {
            let result = gc_system.new_object(VMString::new("named"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMRange>() {
            let result = gc_system.new_object(VMString::new("range"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMWrapper>() {
            let result = gc_system.new_object(VMString::new("wrapper"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBoolean>() {
            let result = gc_system.new_object(VMString::new("bool"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMInstructions>() {
            let result = gc_system.new_object(VMString::new("instructions"));
            vm.push_vmobject(result)?;
        } else if ref_obj.isinstance::<VMBytes>() {
            let result = gc_system.new_object(VMString::new("bytes"));
            vm.push_vmobject(result)?;
        } else {
            let result = gc_system.new_object(VMString::new(""));
            vm.push_vmobject(result)?;
        }
        ref_obj.drop_ref();
        Ok(None)
    }
    pub fn import(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
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
        let vm_instruction_package = bincode::deserialize(&contents);
        if vm_instruction_package.is_err() {
            return Err(VMError::FileError(format!(
                "Cannot deserialize file: {} : {:?}",
                path,
                vm_instruction_package.unwrap_err()
            )));
        }
        let vm_instructions = gc_system.new_object(VMInstructions::new(
            vm_instruction_package.as_ref().unwrap(),
        ));

        vm.push_vmobject(vm_instructions)?;
        path_arg.drop_ref();
        Ok(None)
    }
    pub fn alias(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(name_idx) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let alias = vm
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .vm_instructions_package
            .get_string_pool()[name_idx as usize]
            .clone();
        let obj = &mut vm.pop_object_and_check()?;
        let mut copied = try_copy_as_vmobject(obj, gc_system).map_err(VMError::VMVariableError)?;
        let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
        obj_alias.push(alias);
        vm.push_vmobject(copied)?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn wipe_alias(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let obj = &mut vm.pop_object_and_check()?;
        let mut copied = try_copy_as_vmobject(obj, gc_system).map_err(VMError::VMVariableError)?;
        let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
        obj_alias.clear();
        vm.push_vmobject(copied)?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn alias_of(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut obj = vm.pop_object_and_check()?;
        let obj_alias = try_const_alias_as_vmobject(&obj).map_err(VMError::VMVariableError)?;
        let mut tuple = Vec::new();
        for alias in obj_alias.iter() {
            tuple.push(gc_system.new_object(VMString::new(alias)));
        }
        let mut tuple_refs = tuple.iter_mut().collect();
        let result = gc_system.new_object(VMTuple::new(&mut tuple_refs));
        for alias in tuple.iter_mut() {
            alias.drop_ref();
        }
        vm.push_vmobject(result)?;
        obj.drop_ref();
        Ok(None)
    }

    pub fn swap(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset_1) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let OpcodeArgument::Int64(offset_2) = opcode.operand2 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };

        if offset_1 < 0 || offset_2 < 0 {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        }
        if offset_1 as usize >= vm.stack.len() || offset_2 as usize >= vm.stack.len() {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        }
        let rev_offset_1 = vm.stack.len() - offset_1 as usize - 1;
        let rev_offset_2 = vm.stack.len() - offset_2 as usize - 1;
        let obj_1 = vm.stack[rev_offset_1 as usize].clone();
        let obj_2 = vm.stack[rev_offset_2 as usize].clone();
        // swap
        vm.stack[rev_offset_1 as usize] = obj_2.clone();
        vm.stack[rev_offset_2 as usize] = obj_1.clone();
        Ok(None)
    }

    pub fn build_set(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut filter = vm.pop_object_and_check()?;
        if !filter.isinstance::<VMLambda>() {
            return Err(VMError::InvalidArgument(
                filter.clone(),
                "BuildSet: Filter requires VMLambda".to_string(),
            ));
        }

        let mut collection = vm.pop_object_and_check()?;
        if collection.isinstance::<VMSet>() {
            return Err(VMError::InvalidArgument(
                collection.clone(),
                "BuildSet: Due to the limitation of the current implementation, the collection cannot be a VMSet. You should build a VMTuple before creating a VMSet".to_string(),
            ));
        }
        if !collection.isinstance::<VMTuple>()
            && !collection.isinstance::<VMString>()
            && !collection.isinstance::<VMBytes>()
            && !collection.isinstance::<VMRange>()
        {
            return Err(VMError::InvalidArgument(
                collection.clone(),
                "BuildSet: Not a collection".to_string(),
            ));
        }

        vm.push_vmobject(gc_system.new_object(VMSet::new(&mut collection, &mut filter)))?;
        collection.drop_ref();
        filter.drop_ref();
        Ok(None)
    }

    pub fn fork_stack_object_ref(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let mut obj = vm.get_object_and_check(offset as usize)?;
        vm.stack.push(VMStackObject::VMObject(obj.clone_ref()));
        Ok(None)
    }

    pub fn push_value_into_tuple(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let mut tuple = vm.get_object_and_check(offset as usize)?;
        let mut obj = vm.pop_object_and_check()?;

        if !tuple.isinstance::<VMTuple>() {
            return Err(VMError::InvalidArgument(
                tuple.clone(),
                "PushValueIntoTuple: Not a tuple".to_string(),
            ));
        }
        let tuple_value = tuple.as_type::<VMTuple>();
        tuple_value
            .append(&mut obj)
            .map_err(|e| VMError::VMVariableError(e))?;
        obj.drop_ref();
        Ok(None)
    }
    pub fn reset_iter(
        vm: &mut VMExecutor,
        _opcode: &ProcessedOpcode,
        _gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let mut obj = vm.get_object_and_check(0)?;
        if obj.isinstance::<VMTuple>() {
            let tuple = obj.as_type::<VMTuple>();
            tuple.reset();
        } else if obj.isinstance::<VMString>() {
            let string = obj.as_type::<VMString>();
            string.reset();
        } else if obj.isinstance::<VMBytes>() {
            let bytes = obj.as_type::<VMBytes>();
            bytes.reset();
        } else if obj.isinstance::<VMRange>() {
            let range = obj.as_type::<VMRange>();
            range.reset();
        } else if obj.isinstance::<VMSet>() {
            let set = obj.as_type::<VMSet>();
            set.reset();
        } else {
            return Err(VMError::InvalidArgument(
                obj.clone(),
                "ResetIter: Not a iterable".to_string(),
            ));
        }
        Ok(None)
    }
    pub fn next_or_jump(
        vm: &mut VMExecutor,
        opcode: &ProcessedOpcode,
        gc_system: &mut GCSystem,
    ) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
        let OpcodeArgument::Int64(offset) = opcode.operand1 else {
            return Err(VMError::InvalidInstruction(opcode.clone()));
        };
        let mut obj = vm.get_object_and_check(0)?;
        if obj.isinstance::<VMTuple>() {
            let tuple = obj.as_type::<VMTuple>();
            let result = tuple.next(gc_system);
            if result.is_none() {
                vm.ip += offset as isize;
            } else {
                vm.push_vmobject(result.unwrap())?;
            }
        } else if obj.isinstance::<VMString>() {
            let tuple = obj.as_type::<VMString>();
            let result = tuple.next(gc_system);
            if result.is_none() {
                vm.ip += offset as isize;
            } else {
                vm.push_vmobject(result.unwrap())?;
            }
        } else if obj.isinstance::<VMBytes>() {
            let tuple = obj.as_type::<VMBytes>();
            let result = tuple.next(gc_system);
            if result.is_none() {
                vm.ip += offset as isize;
            } else {
                vm.push_vmobject(result.unwrap())?;
            }
        } else if obj.isinstance::<VMRange>() {
            let tuple = obj.as_type::<VMRange>();
            let result = tuple.next(gc_system);
            if result.is_none() {
                vm.ip += offset as isize;
            } else {
                vm.push_vmobject(result.unwrap())?;
            }
        } else if obj.isinstance::<VMSet>() {
            let tuple = obj.as_type::<VMSet>();
            let result = tuple.next(gc_system);
            if result.is_none() {
                vm.ip += offset as isize;
            } else {
                vm.push_vmobject(result.unwrap())?;
            }
        } else {
            return Err(VMError::InvalidArgument(
                obj.clone(),
                "NextOrJump: Not a iterable".to_string(),
            ));
        }

        Ok(None)
    }
}
