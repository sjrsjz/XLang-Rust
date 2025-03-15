use std::collections::HashMap;
use std::fmt::format;

use crate::try_ref_as_type;
use crate::vm::ir::DebugInfo;
use crate::vm::ir::IROperation;
use crate::vm::ir::IR;

use super::super::gc::gc::*;
use super::context::*;
use super::variable::*;

#[derive(Debug)]
pub enum VMError {
    InvaildInstruction(IR),
    TryEnterNotLambda(GCRef),
    EmptyStack,
    ArgumentIsNotTuple(GCRef),
    UnableToReference(GCRef),
    NotVMObject(VMStackObject),
    ContextError(ContextError),
    VMVariableError(VMVariableError),
    AssertFailed,
    CannotGetSelf(GCRef),
    InvaildArgument(GCRef, String),
}

#[derive(Debug)]
pub struct IRExecutor {
    context: Context,
    stack: Vec<VMStackObject>,
    ip: isize,
    lambda_instructions: Vec<GCRef>,
    original_code: Option<String>,
    debug_info: Option<DebugInfo>,
}

mod native_functions{
    use crate::vm::{executor::variable::{try_ref_as_vmobject, try_repr_vmobject, VMNull, VMTuple, VMVariableError}, gc::gc::{GCRef, GCSystem}};

    pub fn print(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        if !tuple.isinstance::<VMTuple>() {
            return Err(VMVariableError::TypeError(tuple, "print's argument must be a tuple".to_string()));
        }
        let tuple = tuple.as_type::<VMTuple>();
        let mut result = String::new();
        for obj in tuple.values.iter() {
            let obj_ref = try_ref_as_vmobject(obj.clone())?;
            let repr = try_repr_vmobject(obj_ref)?;
            result.push_str(&format!("{}, ", repr));
        }
        result = result.trim_end_matches(", ").to_string();
        println!("{}", result);
        let obj = gc_system.new_object(VMNull::new());
        return Ok(obj);
    }
}


impl IRExecutor {
    pub fn new(original_code: Option<String>) -> Self {
        IRExecutor {
            context: Context::new(),
            stack: Vec::new(),
            ip: 0,
            lambda_instructions: Vec::new(),
            original_code: original_code,
            debug_info: None,
        }
    }
    pub fn set_debug_info(&mut self, debug_info: DebugInfo) {
        self.debug_info = Some(debug_info);
    }

    pub fn pop_object(&mut self) -> Result<VMStackObject, VMError> {
        if self.stack.len() == 0 {
            return Err(VMError::EmptyStack);
        }
        Ok(self.stack.pop().unwrap())
    }


    pub fn inject_builtin_functions(
        context:&mut Context,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        let mut built_in_functions:HashMap<String,GCRef> = HashMap::new();
        built_in_functions.insert("print".to_string(), gc_system.new_object(VMNativeFunction::new(native_functions::print)));
        for (name, func) in built_in_functions.iter() {
            let result = context.let_var(name.clone(), func.clone(), gc_system);
            func.offline();
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }
        Ok(())
    }

}
impl IRExecutor {
    pub fn enter_lambda(
        &mut self,
        lambda_object: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        if !lambda_object.isinstance::<VMLambda>() {
            return Err(VMError::TryEnterNotLambda(lambda_object));
        }

        let lambda = lambda_object.as_type::<VMLambda>();

        let use_new_instructions = self.lambda_instructions.len() == 0
            || lambda.lambda_instructions != *self.lambda_instructions.last().unwrap();
        if use_new_instructions {
            self.lambda_instructions
                .push(lambda.lambda_instructions.clone());
        }
        self.stack
            .push(VMStackObject::LastIP(self.ip  as usize, use_new_instructions));
        self.context
            .new_frame(&self.stack, true, lambda.code_position, false);

        return Ok(());
    }

    pub fn execute(
        &mut self,
        lambda_object: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMError> {
        self.enter_lambda(lambda_object.clone(), gc_system)?;

        self.ip = self
            .lambda_instructions
            .last()
            .unwrap()
            .as_const_type::<VMInstructions>()
            .func_ips
            .get(&lambda_object.as_const_type::<VMLambda>().signature)
            .unwrap()
            .clone() as isize;

        //create builtin functions
        IRExecutor::inject_builtin_functions(&mut self.context, gc_system)?;

        //run!
        while self.lambda_instructions.len() > 0
            && self.ip
                < self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .instructions
                    .len() as isize
        {
            let instruction = self
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .instructions[self.ip as usize]
                .clone();

            println!("{}: {:?}", self.ip, instruction); // debug
            self.execute_instruction(instruction, gc_system)?;
            gc_system.collect(); // debug
            //gc_system.print_reference_graph(); // debug
            self.ip += 1;
        }
        let result = self
            .stack
            .pop();
        if result.is_none() {
            return Err(VMError::EmptyStack);
        }
        let result = result.unwrap();
        match result {
            VMStackObject::VMObject(obj) => {
                return Ok(obj);
            }
            _ => {
                return Err(VMError::NotVMObject(result));
            }            
        }
    }
}

impl IRExecutor {

    pub fn is_variable(&self, object: &GCRef) -> bool{
        if object.isinstance::<VMVariableWrapper>() {
            return true;
        }
        return false;
    }

    pub fn offline_if_not_variable(&self, object: &GCRef) {
        if !self.is_variable(object) {
            object.offline();
        }
    }

    pub fn execute_instruction(
        &mut self,
        instruction: IR,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        match &instruction {
            IR::LoadInt(value) => {
                let obj = gc_system.new_object(VMInt::new(*value));
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::LoadFloat(value) => {
                let obj = gc_system.new_object(VMFloat::new(*value));
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::LoadString(value) => {
                let obj = gc_system.new_object(VMString::new(value.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::LoadBool(value) => {
                let obj = gc_system.new_object(VMBoolean::new(*value));
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::LoadNull => {
                let obj = gc_system.new_object(VMNull::new());
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::LoadLambda(signature, code_position) => {
                let default_args = self.pop_object()?;
                if let VMStackObject::VMObject(default_args_tuple) = default_args {
                    let default_args_tuple_ref = try_ref_as_vmobject(default_args_tuple.clone());
                    if default_args_tuple_ref.is_err() {
                        return Err(VMError::UnableToReference(default_args_tuple));
                    }
                    let default_args_tuple_ref = default_args_tuple_ref.unwrap();
                    if !default_args_tuple_ref.isinstance::<VMTuple>() {
                        return Err(VMError::ArgumentIsNotTuple(default_args_tuple_ref));
                    }

                    let obj = gc_system.new_object(VMLambda::new(
                        *code_position,
                        signature.clone(),
                        default_args_tuple_ref.clone(),
                        None,
                        self.lambda_instructions.last().unwrap().clone(),
                    ));
                    self.stack.push(VMStackObject::VMObject(obj));
                    default_args_tuple.offline(); // offline the tuple because it is not stored in context
                } else {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                }
            }
            IR::BuildTuple(size) => {
                let mut tuple: Vec<GCRef> = Vec::new();
                let mut tuple_original = Vec::new();
                for _ in 0..*size {
                    let obj = self.pop_object()?;
                    if let VMStackObject::VMObject(obj) = obj {
                        tuple_original.insert(0, obj.clone());
                        let obj_ref = try_ref_as_vmobject(obj.clone());
                        if obj_ref.is_err() {
                            return Err(VMError::UnableToReference(obj));
                        }
                        tuple.insert(0, obj_ref.unwrap());
                    } else {
                        return Err(VMError::InvaildInstruction(instruction.clone()));
                    }
                }
                let obj = gc_system.new_object(VMTuple::new(tuple.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                for obj in tuple_original {
                    self.offline_if_not_variable(&obj);
                }
            }

            IR::BuildKeyValue => {
                let value_original = self.pop_object()?;
                let key_original = self.pop_object()?;
                let VMStackObject::VMObject(value_original) = value_original else {
                    return Err(VMError::NotVMObject(value_original));
                };
                let VMStackObject::VMObject(key_original) = key_original else {
                    return Err(VMError::NotVMObject(key_original));
                };
                
                let value_ref = try_ref_as_vmobject(value_original.clone());
                if value_ref.is_err() {
                    return Err(VMError::UnableToReference(value_original));
                }
                let value = value_ref.unwrap();
                let key_ref = try_ref_as_vmobject(key_original.clone());
                if key_ref.is_err() {
                    return Err(VMError::UnableToReference(key_original));
                }
                let key = key_ref.unwrap();
                let obj = gc_system.new_object(VMKeyVal::new(key.clone(), value.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&key_original);
                self.offline_if_not_variable(&value_original);
            }

            IR::BuildNamed => {
                let value_original = self.pop_object()?;
                let key_original = self.pop_object()?;
                let VMStackObject::VMObject(value_original) = value_original else {
                    return Err(VMError::NotVMObject(value_original));
                };
                let VMStackObject::VMObject(key_original) = key_original else {
                    return Err(VMError::NotVMObject(key_original));
                };
                
                let value_ref = try_ref_as_vmobject(value_original.clone());
                if value_ref.is_err() {
                    return Err(VMError::UnableToReference(value_original));
                }
                let value = value_ref.unwrap();
                let key_ref = try_ref_as_vmobject(key_original.clone());
                if key_ref.is_err() {
                    return Err(VMError::UnableToReference(key_original));
                }
                let key = key_ref.unwrap();
                let obj = gc_system.new_object(VMNamed::new(key.clone(), value.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&key_original);
                self.offline_if_not_variable(&value_original);
            }


            IR::BinaryOp(operation) => {
                let right_original = self.pop_object()?;
                let right_original = match right_original {
                    VMStackObject::VMObject(right_original) => right_original,
                    _ => return Err(VMError::NotVMObject(right_original)),
                };

                let left_original = self.pop_object()?;
                let left_original = match left_original {
                    VMStackObject::VMObject(left_original) => left_original,
                    _ => return Err(VMError::NotVMObject(left_original)),
                };
                let left_ref = try_ref_as_vmobject(left_original.clone());
                if left_ref.is_err() {
                    return Err(VMError::UnableToReference(left_original));
                }
                let left = left_ref.unwrap();
                let right_ref = try_ref_as_vmobject(right_original.clone());
                if right_ref.is_err() {
                    return Err(VMError::UnableToReference(right_original));
                }
                let right = right_ref.unwrap();

                let obj = match operation {
                    IROperation::Equal =>{
                        gc_system.new_object(VMBoolean::new(try_eq_as_vmobject(left, right)))
                    }
                    IROperation::NotEqual => {
                        gc_system.new_object(VMBoolean::new(!try_eq_as_vmobject(left, right)))
                    }
                    IROperation::Greater => {
                        let result = try_greater_than_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(result.unwrap()))
                    }
                    IROperation::Less => {
                        let result = try_less_than_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(result.unwrap()))
                    }
                    IROperation::GreaterEqual => {
                        let result = try_less_than_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(!result.unwrap()))
                    }
                    IROperation::LessEqual => {
                        let result = try_greater_than_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(!result.unwrap()))
                    }

                    IROperation::Add => {
                        let result = try_add_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::Subtract => {
                        let result = try_sub_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::Multiply => {
                        let result = try_mul_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::Divide => {
                        let result = try_div_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::Modulus => {
                        let result = try_mod_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::BitwiseAnd => {
                        let result = try_bitwise_and_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::BitwiseOr => {
                        let result = try_bitwise_or_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::BitwiseXor => {
                        let result = try_bitwise_xor_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::ShiftLeft => {
                        let result = try_shift_left_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }
                    IROperation::ShiftRight => {
                        let result = try_shift_right_as_vmobject(left, right, gc_system);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        result.unwrap()
                    }

                    IROperation::And => {
                        let result = try_and_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(result.unwrap()))
                    }

                    IROperation::Or => {
                        let result = try_or_as_vmobject(left, right);
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(result.unwrap()))
                    }
                    _ => return Err(VMError::InvaildInstruction(instruction.clone())),
                };
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&left_original);
                self.offline_if_not_variable(&right_original);
            }

            IR::UnaryOp(operation) => {
                let original = self.pop_object()?;
                let original = match original {
                    VMStackObject::VMObject(original) => original,
                    _ => return Err(VMError::NotVMObject(original)),
                };
                let ref_obj = try_ref_as_vmobject(original.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(original));
                }
                let obj = match operation {
                    IROperation::Not => {
                        let result = try_not_as_vmobject(ref_obj.unwrap());
                        if result.is_err() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(result.unwrap()))
                    }
                    IROperation::Subtract => {
                        let ref_obj = ref_obj.unwrap();
                        if ref_obj.isinstance::<VMInt>() {
                            let value = ref_obj.as_const_type::<VMInt>().value;
                            gc_system.new_object(VMInt::new(-value))
                        } else if ref_obj.isinstance::<VMFloat>() {
                            let value = ref_obj.as_const_type::<VMFloat>().value;
                            gc_system.new_object(VMFloat::new(-value))
                        } else {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                    }
                    _ => return Err(VMError::InvaildInstruction(instruction.clone())),
                };
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&original);
            }

            IR::Let(name) => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let obj_ref = try_ref_as_vmobject(obj.clone());
                if obj_ref.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let obj_ref = obj_ref.unwrap();
                let result = self.context.let_var(name.clone(), obj_ref.clone(), gc_system);
                if result.is_err(){
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
                self.stack.push(VMStackObject::VMObject(obj));
            }

            IR::Get(name) => {
                let obj = self.context.get_var(name);
                if obj.is_err() {
                    return Err(VMError::ContextError(obj.unwrap_err()));
                }
                let obj = obj.unwrap();
                self.stack.push(VMStackObject::VMObject(obj));
            }

            IR::Set => {
                let value = self.pop_object()?;
                let value = match value {
                    VMStackObject::VMObject(value) => value,
                    _ => return Err(VMError::NotVMObject(value)),
                };
                let value_ref = try_ref_as_vmobject(value.clone());
                if value_ref.is_err() {
                    return Err(VMError::UnableToReference(value));
                }
                let value_ref = value_ref.unwrap();
                let reference = self.pop_object()?;
                let reference = match reference {
                    VMStackObject::VMObject(reference) => reference,
                    _ => return Err(VMError::NotVMObject(reference)),
                };
                let result = try_assgin_as_vmobject(reference, value_ref);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&value);
            }

            IR::Return => {
                if self.stack.len() < *self.context.stack_pointers.last().unwrap() {
                    return Err(VMError::EmptyStack);
                }
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                self.stack.truncate(*self.context.stack_pointers.last().unwrap());
                let ip_info = self.stack.pop().unwrap();
                let VMStackObject::LastIP(ip, use_new_instructions) = ip_info else {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                };
                self.ip = ip as isize;
                if use_new_instructions {
                    self.lambda_instructions.pop();
                }
                let result = self.context.pop_frame(&mut self.stack, true);
                if result.is_err() {
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
                self.stack.push(VMStackObject::VMObject(obj));
            }

            IR::NewFrame => {
                self.context.new_frame(&mut self.stack, false, 0, false);
            }
            IR::PopFrame => {
                let result = self.context.pop_frame(&mut self.stack, false);
                if result.is_err() {
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
            }
            IR::JumpOffset(offset) => {
                self.ip += offset;
            }
            IR::JumpIfFalseOffset(offset) => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();
                if !ref_obj.isinstance::<VMBoolean>() {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                }
                if !ref_obj.as_const_type::<VMBoolean>().value {
                    self.ip += offset;
                }
                self.offline_if_not_variable(&obj);
            }
            IR::ResetStack => {
                for i in *self.context.stack_pointers.last().unwrap()..self.stack.len() {
                    let obj = self.stack[i].clone();
                    if let VMStackObject::VMObject(obj) = obj {
                        self.offline_if_not_variable(&obj);
                    }
                }
                self.stack.truncate(*self.context.stack_pointers.last().unwrap());
            }
            IR::GetAttr => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();

                let attr = self.pop_object()?;
                let attr = match attr {
                    VMStackObject::VMObject(attr) => attr,
                    _ => return Err(VMError::NotVMObject(attr)),
                };
                let attr_ref = try_ref_as_vmobject(attr.clone());
                if attr_ref.is_err() {
                    return Err(VMError::UnableToReference(attr));
                }
                let attr_ref = attr_ref.unwrap();
                let result = try_get_attr_as_vmobject(ref_obj, attr_ref);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
                self.offline_if_not_variable(&attr);
            }

            IR::IndexOf => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();

                let index = self.pop_object()?;
                let index = match index {
                    VMStackObject::VMObject(index) => index,
                    _ => return Err(VMError::NotVMObject(index)),
                };
                let index_ref = try_ref_as_vmobject(index.clone());
                if index_ref.is_err() {
                    return Err(VMError::UnableToReference(index));
                }
                let index_ref = index_ref.unwrap();
                let result = try_index_of_as_vmobject(ref_obj, index_ref, gc_system);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
                self.offline_if_not_variable(&index);
            }

            IR::KeyOf => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();
                let result = try_key_of_as_vmobject(ref_obj);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
            }

            IR::ValueOf => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();
                let result = try_value_of_as_vmobject(ref_obj);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
            }

            IR::Assert => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };
                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();
                if !ref_obj.isinstance::<VMBoolean>() {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                }
                if !ref_obj.as_const_type::<VMBoolean>().value {
                    return Err(VMError::AssertFailed);
                }
                self.offline_if_not_variable(&obj);
            }

            IR::SelfOf => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };

                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();

                if !ref_obj.isinstance::<VMLambda>() {
                    return Err(VMError::CannotGetSelf(obj));
                }
                let lambda = ref_obj.as_const_type::<VMLambda>();
                let self_obj = lambda.self_object.clone();
                match self_obj {
                    Some(self_obj) => {
                        let self_obj_ref = try_ref_as_vmobject(self_obj.clone());
                        if self_obj_ref.is_err() {
                            return Err(VMError::UnableToReference(self_obj));
                        }
                        let self_obj_ref = self_obj_ref.unwrap();
                        self.stack.push(VMStackObject::VMObject(self_obj_ref));
                    }
                    None => {
                        self.stack.push(VMStackObject::VMObject(gc_system.new_object(VMNull::new())));
                    }
                }
                self.offline_if_not_variable(&obj);
            }

            IR::CopyValue => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };

                let ref_obj = try_ref_as_vmobject(obj.clone());
                if ref_obj.is_err() {
                    return Err(VMError::UnableToReference(obj));
                }
                let ref_obj = ref_obj.unwrap();

                let result = try_copy_as_vmobject(ref_obj, gc_system);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));


                self.offline_if_not_variable(&obj);
            }

            IR::RefValue => {
                return Err(VMError::InvaildInstruction(instruction.clone()))
            }

            IR::DerefValue => {
                return Err(VMError::InvaildInstruction(instruction.clone()));
            }


            IR::DebugInfo(debug_info) => {
                if self.original_code.is_some() {
                    let debug_info = debug_info.clone();
                    self.set_debug_info(debug_info);
                }
            }


            IR::CallLambda => {
                let arg_tuple = self.pop_object()?;
                let lambda = self.pop_object()?;
                let arg_tuple = match arg_tuple {
                    VMStackObject::VMObject(arg_tuple) => arg_tuple,
                    _ => return Err(VMError::NotVMObject(arg_tuple)),
                };
                let lambda = match lambda {
                    VMStackObject::VMObject(lambda) => lambda,
                    _ => return Err(VMError::NotVMObject(lambda)),
                };
                let arg_tuple_ref = try_ref_as_vmobject(arg_tuple.clone());
                if arg_tuple_ref.is_err() {
                    return Err(VMError::UnableToReference(arg_tuple));
                }
                let arg_tuple_ref = arg_tuple_ref.unwrap();
                if !arg_tuple_ref.isinstance::<VMTuple>() {
                    return Err(VMError::ArgumentIsNotTuple(arg_tuple_ref));
                }
                let lambda_ref = try_ref_as_vmobject(lambda.clone());
                if lambda_ref.is_err() {
                    debug_print_repr(lambda.clone());
                    return Err(VMError::UnableToReference(lambda));
                }
                let lambda_ref = lambda_ref.unwrap();

                if lambda_ref.isinstance::<VMNativeFunction>() {
                    let lambda_ref = lambda_ref.as_const_type::<VMNativeFunction>();
                    let result = lambda_ref.call(arg_tuple.clone(), gc_system);
                    if result.is_err() {
                        return Err(VMError::VMVariableError(result.unwrap_err()));
                    }
                    let result = result.unwrap();
                    self.stack.push(VMStackObject::VMObject(result));
                    self.offline_if_not_variable(&arg_tuple);
                    self.offline_if_not_variable(&lambda);
                    return Ok(());
                }

                if !lambda_ref.isinstance::<VMLambda>() {
                    return Err(VMError::TryEnterNotLambda(lambda_ref));
                }
                self.enter_lambda(lambda_ref.clone(), gc_system)?;

                let lambda_ref = lambda_ref.as_const_type::<VMLambda>();


                let signature = lambda_ref.signature.clone();
                let default_args = lambda_ref.default_args_tuple.clone();

                let result = default_args.as_type::<VMTuple>().assgin_members(arg_tuple.clone());
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }

                for v in default_args.as_type::<VMTuple>().values.iter() {
                    let v_ref = try_ref_as_vmobject(v.clone());
                    if v_ref.is_err() {
                        return Err(VMError::UnableToReference(v.clone()));
                    }
                    let v_ref = v_ref.unwrap();

                    if !v_ref.isinstance::<VMNamed>() {
                        return Err(VMError::InvaildArgument(v.clone(), format!("Not a VMNamed in Lambda arguments: {:?}", v)));
                    }
                    let v_ref = v_ref.as_const_type::<VMNamed>();
                    let name = v_ref.key.clone();
                    let value = v_ref.value.clone();

                    if !name.isinstance::<VMString>() {
                        return Err(VMError::InvaildArgument(name, "Not a VMString".to_string()));
                    }
                    let name = name.as_const_type::<VMString>();
                    let name = name.value.clone();
                    let value_ref = try_ref_as_vmobject(value.clone());
                    if value_ref.is_err() {
                        return Err(VMError::UnableToReference(value));
                    }
                    let value_ref = value_ref.unwrap();
                    let result = self.context.let_var(name.clone(), value_ref.clone(), gc_system);
                    if result.is_err() {
                        return Err(VMError::ContextError(result.unwrap_err()));
                    }
                }

                if lambda_ref.self_object.is_some() {
                    let self_obj = lambda_ref.self_object.clone().unwrap();
                    let self_obj_ref = try_ref_as_vmobject(self_obj.clone());
                    if self_obj_ref.is_err() {
                        return Err(VMError::UnableToReference(self_obj));
                    }
                    let self_obj_ref = self_obj_ref.unwrap();
                    let result = self.context.let_var("self".to_string(), self_obj_ref.clone(), gc_system);
                    if result.is_err() {
                        return Err(VMError::ContextError(result.unwrap_err()));
                    }
                }
                let func_ips = &self.lambda_instructions.last().unwrap().as_const_type::<VMInstructions>().func_ips;
                let ip = func_ips
                    .get(&signature)
                    .unwrap()
                    .clone() as isize;
                self.ip = ip - 1;

                self.offline_if_not_variable(&arg_tuple);
                self.offline_if_not_variable(&lambda);
            }


            _ => return Err(VMError::InvaildInstruction(instruction.clone())),
        }
        Ok(())
    }
}
