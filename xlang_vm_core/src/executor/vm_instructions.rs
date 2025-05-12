use crate::executor::context::ContextFrameType;
use crate::executor::variable::*;
use crate::executor::vm::VMError;
use crate::executor::vm::VMExecutor;
use crate::gc::GCSystem;
use crate::opcode::{OpcodeArgument, ProcessedOpcode};
use std::fs::File;
use std::io::Read;

use crate::executor::vm::SpawnedCoroutine;

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
            let mut instruction = vm.get_object_and_check(0)?;
            let mut idx = 1;
            let mut capture = None;
            let mut dynamic_params = false;
            if let OpcodeArgument::Int32(flags) = opcode.operand3 {
                if flags & 1 != 0 {
                    capture = Some(vm.get_object_and_check(1)?);
                    idx = 2;
                }
                if flags & 2 != 0 {
                    dynamic_params = true;
                }
            }
            let mut default_args_tuple = vm.get_object_and_check(idx)?;

            if !instruction.isinstance::<VMInstructions>()
                && !instruction.isinstance::<VMCLambdaInstruction>()
            {
                return Err(VMError::InvalidArgument(
                    instruction.clone_ref(),
                    "LoadLambda requires a VMInstructions or VMCLambdaInstruction".to_string(),
                ));
            }
            if !default_args_tuple.isinstance::<VMTuple>() {
                return Err(VMError::ArgumentIsNotTuple(default_args_tuple.clone_ref()));
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
                &mut default_args_tuple,
                capture.as_mut(),
                None,
                &mut VMLambdaBody::VMInstruction(instruction.clone()),
                &mut lambda_result,
                dynamic_params,
            ));
            // Pop objects from stack after successful operation
            if capture.is_some() {
                vm.pop_object()?;
            }
            vm.pop_object()?;
            vm.pop_object()?;
            vm.push_vmobject(obj)?;

            // Drop references at the end
            default_args_tuple.drop_ref();
            lambda_result.drop_ref();
            instruction.drop_ref();
            if let Some(mut capture) = capture {
                capture.drop_ref();
            }
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
        for i in 0..size {
            let obj = vm.get_object_and_check((size - 1 - i) as usize)?; // Changed: Access in reverse order
            tuple.push(obj);
        }
        let mut new_refs = tuple.iter_mut().collect();
        let obj = gc_system.new_object(VMTuple::new(&mut new_refs));

        // Pop objects from stack after successful operation
        for _ in 0..size {
            vm.pop_object()?;
        }
        vm.push_vmobject(obj)?;

        // Drop references at the end
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
    let mut obj = vm.get_object_and_check(0)?;
    if !obj.isinstance::<VMTuple>() && !obj.isinstance::<VMKeyVal>(){
        return Err(VMError::VMVariableError(VMVariableError::TypeError(
            obj.clone_ref(),
            "Bind requires a VMTuple or VMKeyVal".to_string(),
        )));
    }
    if obj.isinstance::<VMKeyVal>() {
        let keyval = obj.as_type::<VMKeyVal>();
        if !keyval.value.isinstance::<VMLambda>() && !keyval.value.isinstance::<VMTuple>() {
            return Err(VMError::VMVariableError(VMVariableError::TypeError(
                keyval.value.clone_ref(),
                "Bind's value requires a VMLambda or VMTuple".to_string(),
            )));
        }
        if keyval.value.isinstance::<VMTuple>() {
            keyval.value.as_type::<VMTuple>().bind_lambda_self(&mut keyval.key);
        } else {
            keyval.value.as_type::<VMLambda>().set_self_object(&mut keyval.key);
        }
        vm.pop_object()?;
        vm.push_vmobject(keyval.value.clone_ref())?;
        obj.drop_ref();
        return Ok(None)
    }
    let mut copied = try_copy_as_vmobject(&mut obj, gc_system).map_err(VMError::VMVariableError)?;
    VMTuple::set_lambda_self(&mut copied);

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(copied)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}

pub fn build_keyval(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut value = vm.get_object_and_check(0)?;
    let mut key = vm.get_object_and_check(1)?;
    let obj = gc_system.new_object(VMKeyVal::new(&mut key, &mut value));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    key.drop_ref();
    value.drop_ref();
    Ok(None)
}
pub fn build_named(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut value = vm.get_object_and_check(0)?;
    let mut key = vm.get_object_and_check(1)?;
    let obj = gc_system.new_object(VMNamed::new(&mut key, &mut value));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
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
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj =
        try_add_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_subtract(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_sub_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_multiply(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_mul_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_divide(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_div_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_modulus(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_mod_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_power(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_power_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_bitwise_and(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_and_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_bitwise_or(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_or_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_bitwise_xor(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = try_xor_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_shift_left(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj =
        try_shift_left_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_shift_right(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj =
        try_shift_right_as_vmobject(&mut left, &mut right, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_equal(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = gc_system.new_object(VMBoolean::new(try_eq_as_vmobject(&left, &right)));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_not_equal(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let obj = gc_system.new_object(VMBoolean::new(!try_eq_as_vmobject(&left, &right)));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_greater(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let result = try_greater_than_as_vmobject(&mut left, &mut right).map_err(VMError::VMVariableError)?;
    let obj = gc_system.new_object(VMBoolean::new(result));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_less(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let result = try_less_than_as_vmobject(&mut left, &mut right).map_err(VMError::VMVariableError)?;
    let obj = gc_system.new_object(VMBoolean::new(result));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_greater_equal(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let result = try_less_than_as_vmobject(&mut left, &mut right).map_err(VMError::VMVariableError)?;
    let obj = gc_system.new_object(VMBoolean::new(!result));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn binary_less_equal(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut right = vm.get_object_and_check(0)?;
    let mut left = vm.get_object_and_check(1)?;

    let result = try_greater_than_as_vmobject(&mut left, &mut right).map_err(VMError::VMVariableError)?;
    let obj = gc_system.new_object(VMBoolean::new(!result));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop references at the end
    left.drop_ref();
    right.drop_ref();
    Ok(None)
}

pub fn unary_minus(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut ref_obj = vm.get_object_and_check(0)?;

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

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop reference at the end
    ref_obj.drop_ref();
    Ok(None)
}

pub fn unary_plus(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut ref_obj = vm.get_object_and_check(0)?;

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

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop reference at the end
    ref_obj.drop_ref();
    Ok(None)
}

pub fn unary_bitwise_not(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut ref_obj = vm.get_object_and_check(0)?;

    let obj = try_not_as_vmobject(&mut ref_obj, gc_system).map_err(VMError::VMVariableError)?;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(obj)?;

    // Drop reference at the end
    ref_obj.drop_ref();
    Ok(None)
}
pub fn let_var(
    vm: &mut VMExecutor,
    opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    if let OpcodeArgument::Int64(name_idx) = opcode.operand1 {
        let mut obj = vm.get_object_and_check(0)?;
        let name = &vm
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .vm_instructions_package
            .get_string_pool()[name_idx as usize];
        let result = vm.context.let_var(name, &mut obj, gc_system);
        if result.is_err() {
            return Err(VMError::ContextError(result.unwrap_err()));
        }
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
    let mut value = vm.get_object_and_check(0)?;
    let mut reference = vm.get_object_and_check(1)?;
    let result =
        try_assign_as_vmobject(&mut reference, &mut value).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(result.clone_ref())?;

    // Drop references at the end
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
    let mut obj = vm.get_object_and_check(0)?;
    // obj is still on top after pop_frame_until_function adjusted the stack
    // let obj = &mut vm.pop_object_and_check()?; // Original logic assumed obj was popped by pop_frame

    let ip_info = vm.stack[vm.stack.len() - 2].clone();
    let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info else {
        // If popping LastIP fails, push obj back before erroring? Or let error handle cleanup?
        // Assuming error handles cleanup for now.
        return Err(VMError::EmptyStack);
    };
    vm.ip = ip as isize;
    let lambda_obj: &mut VMLambda = self_lambda.as_type::<VMLambda>();
    lambda_obj.set_result(&mut obj);

    // Pop the original obj from stack (which was obtained via get_object_and_check)
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj.clone_ref())?; // Push the result back

    // Drop references at the end
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
    // obj is still on top after pop_frame_until_boundary adjusted the stack
    let obj = vm.get_object_and_check(0)?;

    let ip_info = vm.stack[vm.stack.len() - 2].clone();
    let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info else {
        return Err(VMError::EmptyStack);
    };
    vm.ip = ip as isize;

    // Pop the original obj from stack
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(obj.clone())?; // Push the exception object back

    // Drop references at the end
    // obj.drop_ref(); // obj was cloned and pushed, original ref count remains
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
    let mut obj = vm.get_object_and_check(0)?;
    vm.entry_lambda.as_type::<VMLambda>().set_result(&mut obj);

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(obj.clone_ref())?;

    // Drop reference at the end
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
    let mut obj = vm.get_object_and_check(0)?;
    if !obj.isinstance::<VMLambda>() {
        return Err(VMError::InvalidArgument(
            obj.clone_ref(), 
            "Await: Not a lambda".to_string(),
        ));
    }
    let lambda = obj.as_const_type::<VMLambda>();
    let is_finished = lambda.coroutine_status == VMCoroutineStatus::Finished;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(gc_system.new_object(VMBoolean::new(is_finished)))?;

    // Drop reference at the end
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
    let obj = vm.get_object_and_check(0)?;
    // obj is still on top after pop_frame_except_top

    let ip_info = vm.stack[vm.stack.len() - 2].clone();
    if let VMStackObject::LastIP(mut self_lambda, ip, use_new_instructions) = ip_info {
        vm.ip = ip as isize;

        // Pop the original obj from stack
        vm.pop_object()?;
        vm.pop_object()?; // Pop the LastIP
        vm.push_vmobject(obj.clone())?; // Push the object back

        if use_new_instructions {
            let poped = vm.lambda_instructions.pop();
            poped.unwrap().drop_ref();
        }
        self_lambda.drop_ref();
    } else {
        // Need to handle the case where obj was retrieved but LastIP wasn't found
        // Maybe push obj back before erroring? For now, assume error handles cleanup.
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
    let mut obj = vm.pop_object_and_check()?; // Changed from pop_object
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
    let mut obj = vm.get_object_and_check(0)?;
    if !obj.isinstance::<VMBoolean>() {
        return Err(VMError::VMVariableError(VMVariableError::TypeError(
            obj.clone_ref(),
            "JumpIfFalseOffset: Not a boolean".to_string(),
        )));
    }
    let jump = !obj.as_const_type::<VMBoolean>().value;

    // Pop object from stack after successful check
    vm.pop_object()?;

    if jump {
        vm.ip += offset as isize;
    }

    // Drop reference at the end
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
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut attr = vm.get_object_and_check(0)?;
    let mut obj = vm.get_object_and_check(1)?;

    let result = try_get_attr_as_vmobject(&mut obj, &mut attr, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop references at the end
    obj.drop_ref();
    attr.drop_ref();
    Ok(None)
}
pub fn index_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut index = vm.get_object_and_check(0)?;
    let mut obj = vm.get_object_and_check(1)?;
    let result =
        try_index_of_as_vmobject(&mut obj, &mut index, gc_system).map_err(VMError::VMVariableError)?;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(result)?; // 不clone是因为已经在try_index_of_as_vmobject产生了新的对象

    // Drop references at the end
    obj.drop_ref();
    index.drop_ref();
    Ok(None)
}
pub fn key_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    _gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let result = try_key_of_as_vmobject(&mut obj).map_err(VMError::VMVariableError)?;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result.clone_ref())?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn value_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    _gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let result = try_value_of_as_vmobject(&mut obj).map_err(VMError::VMVariableError)?;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result.clone_ref())?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn assert(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    if !obj.isinstance::<VMBoolean>() {
        return Err(VMError::VMVariableError(VMVariableError::TypeError(
            obj.clone_ref(), // Clone before potential drop
            "Assert: Not a boolean".to_string(),
        )));
    }
    let assert_passed = obj.as_const_type::<VMBoolean>().value;

    // Pop object from stack after successful check
    vm.pop_object()?;
    obj.drop_ref();

    if !assert_passed {
        // Drop ref before erroring if assert fails
        return Err(VMError::AssertFailed);
    }
    vm.push_vmobject(gc_system.new_object(VMBoolean::new(true)))?;
    Ok(None)
}
pub fn self_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;

    if !obj.isinstance::<VMLambda>() {
        return Err(VMError::CannotGetSelf(obj.clone())); // Clone before potential drop
    }
    let lambda = obj.as_type::<VMLambda>();
    let self_obj_opt = lambda.self_object.as_mut();

    // Pop object from stack after successful operation
    vm.pop_object()?;

    match self_obj_opt {
        Some(self_obj) => {
            vm.push_vmobject(self_obj.clone_ref())?;
        }
        None => {
            vm.stack
                .push(VMStackObject::VMObject(gc_system.new_object(VMNull::new())));
        }
    }

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn deepcopy(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let result = try_deepcopy_as_vmobject(&mut obj, gc_system).map_err(|_| {
        VMError::VMVariableError(VMVariableError::TypeError(
            obj.clone_ref(), // Clone before potential drop
            "Not a copyable object".to_string(),
        ))
    })?;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn copy(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let result = try_copy_as_vmobject(&mut obj, gc_system).map_err(|_| {
        VMError::VMVariableError(VMVariableError::TypeError(
            obj.clone_ref(), // Clone before potential drop
            "Not a copyable object".to_string(),
        ))
    })?;

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn call_lambda(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut arg_tuple = vm.get_object_and_check(0)?;
    let mut original_arg_tuple = arg_tuple.clone();
    let mut lambda = vm.get_object_and_check(1)?;

    if !lambda.isinstance::<VMLambda>() {
        return Err(VMError::TryEnterNotLambda(lambda.clone_ref())); // Clone before potential drop
    }

    let lambda_obj = lambda.as_type::<VMLambda>();

    let signature = lambda_obj.signature.clone();
    if lambda_obj.dynamic_params {
        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .assign_members(&mut arg_tuple); // Pass arg_tuple by reference
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }
        arg_tuple = lambda_obj.default_args_tuple.clone_ref();
    } else {
        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .clone_and_assign_members(&mut arg_tuple, gc_system); // Pass arg_tuple by reference
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }
        arg_tuple = result.unwrap();
    }
    let clambda_signature = lambda_obj
        .alias_const()
        .first()
        .unwrap_or(&lambda_obj.signature)
        .clone();
    match lambda_obj.lambda_body {
        VMLambdaBody::VMInstruction(ref mut body) => {
            if body.isinstance::<VMCLambdaInstruction>() {
                let clambda = body.as_type::<VMCLambdaInstruction>();
                let mut result = clambda
                    .call(&clambda_signature, &mut arg_tuple, gc_system)
                    .map_err(|e| {
                        arg_tuple.drop_ref();
                        VMError::VMVariableError(e)
                    })?;
                lambda_obj.set_result(&mut result);

                // Pop objects from stack after successful operation
                vm.pop_object()?;
                vm.pop_object()?;
                vm.push_vmobject(result)?;

                // Drop references at the end
                arg_tuple.drop_ref();
                lambda.drop_ref();
                original_arg_tuple.drop_ref();
                return Ok(None);
            }

            // Pop objects from stack before entering lambda
            vm.pop_object()?; // Pop arg_tuple
            vm.pop_object()?; // Pop lambda

            let enter_result = vm.enter_lambda(&mut lambda, &mut arg_tuple, gc_system); // Pass lambda by reference

            // Drop references after potential enter_lambda
            arg_tuple.drop_ref();
            lambda.drop_ref(); // lambda is now managed by enter_lambda or dropped if error

            enter_result?; // Propagate error from enter_lambda

            let func_ips = &vm
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .vm_instructions_package
                .get_table();
            let ip = *func_ips.get(&signature).unwrap() as isize;
            vm.ip = ip;
            original_arg_tuple.drop_ref();
            Ok(None)
        }
        VMLambdaBody::VMNativeFunction(native_function) => {
            let result = native_function(lambda_obj.self_object.as_mut(), lambda_obj.capture.as_mut(), &mut arg_tuple, gc_system); // Clone arg_tuple for native call
            if result.is_err() {
                arg_tuple.drop_ref();
                return Err(VMError::VMVariableError(result.unwrap_err()));
            }
            let mut result = result.unwrap();
            lambda_obj.set_result(&mut result);

            // Pop objects from stack after successful operation
            vm.pop_object()?;
            vm.pop_object()?;
            vm.push_vmobject(result)?;

            // Drop references at the end
            arg_tuple.drop_ref();
            lambda.drop_ref();
            original_arg_tuple.drop_ref();
            Ok(None)
        }
        VMLambdaBody::VMNativeGeneratorFunction(ref mut generator) => {
            let result = match std::sync::Arc::get_mut(generator) {
                Some(generator) => generator.init(&mut arg_tuple.clone(), gc_system), // Clone arg_tuple
                None => {
                    arg_tuple.drop_ref();
                    return Err(VMError::VMVariableError(VMVariableError::TypeError(
                        lambda.clone_ref(), // Clone before potential drop
                        "Async function is not mutable".to_string(),
                    )));
                }
            };
            if result.is_err() {
                arg_tuple.drop_ref();
                return Err(VMError::VMVariableError(result.unwrap_err()));
            }

            // Pop objects from stack before entering lambda
            vm.pop_object()?; // Pop arg_tuple
            vm.pop_object()?; // Pop lambda

            let enter_result = vm.enter_lambda(&mut lambda, &mut arg_tuple, gc_system); // Pass lambda by reference

            // Drop references after potential enter_lambda
            arg_tuple.drop_ref();
            lambda.drop_ref(); // lambda is now managed by enter_lambda or dropped if error

            enter_result?; // Propagate error from enter_lambda
            original_arg_tuple.drop_ref();
            Ok(None)
        }
    }
}
pub fn async_call_lambda(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut arg_tuple = vm.get_object_and_check(0)?;
    let mut original_arg_tuple = arg_tuple.clone();
    let mut lambda = vm.get_object_and_check(1)?;

    if !lambda.isinstance::<VMLambda>() {
        return Err(VMError::TryEnterNotLambda(lambda.clone_ref())); // Clone before potential drop
    }

    let lambda_obj = lambda.as_type::<VMLambda>();

    if lambda_obj.dynamic_params {
        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .assign_members(&mut arg_tuple); // Pass arg_tuple by reference
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }
        arg_tuple = lambda_obj.default_args_tuple.clone_ref();
    } else {
        let result = lambda_obj
            .default_args_tuple
            .as_type::<VMTuple>()
            .clone_and_assign_members(&mut arg_tuple, gc_system); // Pass arg_tuple by reference
        if result.is_err() {
            return Err(VMError::VMVariableError(result.unwrap_err()));
        }
        arg_tuple = result.unwrap();
    }

    match lambda_obj.lambda_body {
        VMLambdaBody::VMInstruction(ref mut body) => {
            if body.isinstance::<VMCLambdaInstruction>() {
                arg_tuple.drop_ref();
                return Err(VMError::InvalidArgument(
                    arg_tuple.clone_ref(), // Clone before potential drop
                    "Async call not supported for VMCLambdaInstruction".to_string(),
                ));
            }
            let spawned_coroutines = vec![SpawnedCoroutine {
                lambda_ref: lambda.clone_ref(), // Clone lambda for the coroutine
                args: arg_tuple.clone_ref(),
            }];

            // Pop objects from stack after successful operation
            vm.pop_object()?; // Pop arg_tuple
            vm.pop_object()?; // Pop original lambda (before push)
            vm.push_vmobject(lambda.clone())?; // Push the original lambda back onto the stack

            // Drop references at the end
            arg_tuple.drop_ref();
            // lambda.drop_ref(); // Don't drop original lambda, it was pushed back
            original_arg_tuple.drop_ref();
            Ok(Some(spawned_coroutines))
        }
        VMLambdaBody::VMNativeFunction(_) => {
            arg_tuple.drop_ref();
            Err(VMError::InvalidArgument(
                arg_tuple.clone_ref(), // Clone before potential drop
                "Native function cannot be async".to_string(),
            ))
        }
        VMLambdaBody::VMNativeGeneratorFunction(ref mut generator) => {
            match std::sync::Arc::get_mut(generator) {
                Some(generator) => {
                    let result = generator.init(&mut arg_tuple.clone(), gc_system); // Clone arg_tuple
                    if result.is_err() {
                        return Err(VMError::VMVariableError(result.unwrap_err()));
                    }
                }
                None => {
                    arg_tuple.drop_ref();
                    return Err(VMError::VMVariableError(VMVariableError::TypeError(
                        lambda.clone_ref(), // Clone before potential drop
                        "Async function is not mutable".to_string(),
                    )));
                }
            }
            let spawned_coroutines = vec![SpawnedCoroutine {
                lambda_ref: lambda.clone_ref(), // Clone lambda for the coroutine
                args: arg_tuple.clone_ref(),
            }];

            // Pop objects from stack after successful operation
            vm.pop_object()?; // Pop arg_tuple
            vm.pop_object()?; // Pop original lambda (before push)
            vm.push_vmobject(lambda.clone())?; // Push the original lambda back onto the stack

            // Drop references at the end
            arg_tuple.drop_ref();
            // lambda.drop_ref(); // Don't drop original lambda, it was pushed back
            original_arg_tuple.drop_ref();
            Ok(Some(spawned_coroutines))
        }
    }
}
pub fn wrap(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let wrapped = VMWrapper::new(&mut obj); // Pass by reference
    let wrapped = gc_system.new_object(wrapped);

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(wrapped)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}

pub fn is_in(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut container = vm.get_object_and_check(0)?;
    let mut obj = vm.get_object_and_check(1)?;
    let result = try_contains_as_vmobject(&mut container, &mut obj).map_err(|_| {
        VMError::VMVariableError(VMVariableError::TypeError(
            container.clone_ref(), // Clone before potential drop
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
    let mut end = vm.get_object_and_check(0)?;
    let mut start = vm.get_object_and_check(1)?;

    if !start.isinstance::<VMInt>() {
        return Err(VMError::InvalidArgument(
            start.clone_ref(), // Clone before potential drop
            "Start of range is not a VMInt".to_string(),
        ));
    }
    if !end.isinstance::<VMInt>() {
        return Err(VMError::InvalidArgument(
            end.clone_ref(), // Clone before potential drop
            "End of range is not a VMInt".to_string(),
        ));
    }
    let start_ref = start.as_const_type::<VMInt>();
    let end_ref = end.as_const_type::<VMInt>();

    let result = gc_system.new_object(VMRange::new(start_ref.value, end_ref.value));

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop references at the end
    start.drop_ref();
    end.drop_ref();
    Ok(None)
}
pub fn type_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut ref_obj = vm.get_object_and_check(0)?;

    let type_str = if ref_obj.isinstance::<VMInt>() {
        "int"
    } else if ref_obj.isinstance::<VMFloat>() {
        "float"
    } else if ref_obj.isinstance::<VMString>() {
        "string"
    } else if ref_obj.isinstance::<VMBoolean>() {
        "bool"
    } else if ref_obj.isinstance::<VMTuple>() {
        "tuple"
    } else if ref_obj.isinstance::<VMLambda>() {
        "lambda"
    } else if ref_obj.isinstance::<VMNull>() {
        "null"
    } else if ref_obj.isinstance::<VMKeyVal>() {
        "keyval"
    } else if ref_obj.isinstance::<VMNamed>() {
        "named"
    } else if ref_obj.isinstance::<VMRange>() {
        "range"
    } else if ref_obj.isinstance::<VMWrapper>() {
        "wrapper"
    } else if ref_obj.isinstance::<VMInstructions>() {
        "instructions"
    } else if ref_obj.isinstance::<VMBytes>() {
        "bytes"
    } else if ref_obj.isinstance::<VMSet>() {
        // Added VMSet
        "set"
    } else {
        ""
    };
    let result = gc_system.new_object(VMString::new(type_str));

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop reference at the end
    ref_obj.drop_ref();
    Ok(None)
}
pub fn import(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut path_arg = vm.get_object_and_check(0)?;

    if !path_arg.isinstance::<VMString>() {
        return Err(VMError::InvalidArgument(
            path_arg.clone_ref(), // Clone before potential drop
            format!(
                "Import requires VMString but got {}",
                try_repr_vmobject(&mut path_arg, None).unwrap_or(format!("{:?}", path_arg))
            ),
        ));
    }

    let path_ref = path_arg.as_const_type::<VMString>();
    let path = path_ref.value.clone(); // Clone path string
    let path_str = path.as_str(); // Use cloned string

    // Drop path_arg reference early as it's no longer needed after cloning the path
    // vm.pop_object()?; // Pop first
    // path_arg.drop_ref(); // Then drop

    let file = File::open(path_str);
    if file.is_err() {
        return Err(VMError::FileError(format!(
            "Cannot open file: {} : {:?}",
            path_str,
            file.unwrap_err()
        )));
    }
    let mut file = file.unwrap();
    let mut contents = vec![];
    let result = file.read_to_end(&mut contents);
    if result.is_err() {
        return Err(VMError::FileError(format!(
            "Cannot read file: {} : {:?}",
            path_str,
            result.unwrap_err()
        )));
    }
    let vm_instruction_package = bincode::deserialize(&contents);
    if vm_instruction_package.is_err() {
        return Err(VMError::FileError(format!(
            "Cannot deserialize file: {} : {:?}",
            path_str,
            vm_instruction_package.unwrap_err()
        )));
    }
    let vm_instructions = gc_system.new_object(VMInstructions::new(
        vm_instruction_package.as_ref().unwrap(),
    ));

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(vm_instructions)?;

    // Drop reference at the end
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
    let mut obj = vm.get_object_and_check(0)?;
    let mut copied = try_copy_as_vmobject(&mut obj, gc_system).map_err(VMError::VMVariableError)?;
    let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
    obj_alias.push(alias);

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(copied)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn wipe_alias(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let mut copied = try_copy_as_vmobject(&mut obj, gc_system).map_err(VMError::VMVariableError)?;
    let obj_alias = try_alias_as_vmobject(&mut copied).map_err(VMError::VMVariableError)?;
    obj_alias.clear();

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(copied)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn alias_of(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let obj_alias = try_const_alias_as_vmobject(&mut obj).map_err(VMError::VMVariableError)?;
    let mut tuple = Vec::new();
    for alias in obj_alias.iter() {
        tuple.push(gc_system.new_object(VMString::new(alias)));
    }
    let mut tuple_refs = tuple.iter_mut().collect();
    let result = gc_system.new_object(VMTuple::new(&mut tuple_refs));

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop references at the end
    for mut alias_obj in tuple {
        // Changed loop variable name
        alias_obj.drop_ref();
    }
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
    let obj_1 = vm.stack[rev_offset_1].clone();
    let obj_2 = vm.stack[rev_offset_2].clone();
    // swap
    vm.stack[rev_offset_1] = obj_2.clone();
    vm.stack[rev_offset_2] = obj_1.clone();
    Ok(None)
}

pub fn build_set(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut filter = vm.get_object_and_check(0)?;
    let mut collection = vm.get_object_and_check(1)?;

    if !filter.isinstance::<VMLambda>() {
        return Err(VMError::InvalidArgument(
            filter.clone_ref(), // Clone before potential drop
            "BuildSet: Filter requires VMLambda".to_string(),
        ));
    }

    if collection.isinstance::<VMSet>() {
        return Err(VMError::InvalidArgument(
                collection.clone_ref(), // Clone before potential drop
                "BuildSet: Due to the limitation of the current implementation, the collection cannot be a VMSet. You should build a VMTuple before creating a VMSet".to_string(),
            ));
    }
    if !collection.isinstance::<VMTuple>()
        && !collection.isinstance::<VMString>()
        && !collection.isinstance::<VMBytes>()
        && !collection.isinstance::<VMRange>()
    {
        return Err(VMError::InvalidArgument(
            collection.clone_ref(), // Clone before potential drop
            "BuildSet: Not a collection".to_string(),
        ));
    }

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(gc_system.new_object(VMSet::new(&mut collection, &mut filter)))?;

    // Drop references at the end
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
    let mut obj = vm.get_object_and_check(0)?; // Changed: Get the object to push first
    let mut tuple = vm.get_object_and_check(offset as usize)?; // Changed: Get tuple using offset + 1

    if !tuple.isinstance::<VMTuple>() {
        return Err(VMError::InvalidArgument(
            tuple.clone_ref(), // Clone before potential drop
            "PushValueIntoTuple: Not a tuple".to_string(),
        ));
    }
    let tuple_value = tuple.as_type::<VMTuple>();
    tuple_value
        .append(&mut obj) // Pass obj by reference
        .map_err(VMError::VMVariableError)?;

    // Pop the object that was pushed into the tuple
    vm.pop_object()?;

    // Drop reference at the end
    obj.drop_ref();
    // tuple reference remains on stack, no drop here
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
            obj.clone_ref(),
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
            obj.clone_ref(),
            "NextOrJump: Not a iterable".to_string(),
        ));
    }

    Ok(None)
}
pub fn get_lambda_capture(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    if !obj.isinstance::<VMLambda>() {
        return Err(VMError::InvalidArgument(
            obj.clone_ref(), // Clone before potential drop
            "GetLambdaCapture: Not a VMLambda".to_string(),
        ));
    }
    let lambda = obj.as_type::<VMLambda>();
    let result = lambda.get_capture();

    // Pop object from stack after successful operation
    vm.pop_object()?;

    match result {
        Some(result) => {
            vm.push_vmobject(result.clone_ref())?;
        }
        None => {
            vm.stack
                .push(VMStackObject::VMObject(gc_system.new_object(VMNull::new())));
        }
    }
    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}
pub fn get_length(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj = vm.get_object_and_check(0)?;
    let result = try_length_of_as_vmobject(&mut obj).map_err(VMError::VMVariableError)?;
    let result = gc_system.new_object(VMInt::new(result as i64));

    // Pop object from stack after successful operation
    vm.pop_object()?;
    vm.push_vmobject(result)?;

    // Drop reference at the end
    obj.drop_ref();
    Ok(None)
}

pub fn check_is_same_object(
    vm: &mut VMExecutor,
    _opcode: &ProcessedOpcode,
    gc_system: &mut GCSystem,
) -> Result<Option<Vec<SpawnedCoroutine>>, VMError> {
    let mut obj_1 = vm.get_object_and_check(0)?;
    let mut obj_2 = vm.get_object_and_check(1)?;
    let result = obj_1 == obj_2;

    // Pop objects from stack after successful operation
    vm.pop_object()?;
    vm.pop_object()?;
    vm.push_vmobject(gc_system.new_object(VMBoolean::new(result)))?;

    // Drop references at the end
    obj_1.drop_ref();
    obj_2.drop_ref();
    Ok(None)
}