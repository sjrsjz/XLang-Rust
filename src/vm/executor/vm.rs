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
}

#[derive(Debug)]
pub struct IRExecutor {
    context: Context,
    stack: Vec<VMStackObject>,
    ip: usize,
    lambda_instructions: Vec<GCRef>,
    original_code: Option<String>,
    debug_info: Option<DebugInfo>,
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
            .push(VMStackObject::LastIP(self.ip, use_new_instructions));
        self.context
            .new_frame(&self.stack, true, lambda.code_position, false);

        return Ok(());
    }

    pub fn execute(
        &mut self,
        lambda_object: GCRef,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMError> {
        self.enter_lambda(lambda_object, gc_system)?;

        //create builtin functions

        //run!
        let mut result = gc_system.new_object(VMNull::new());

        while self.lambda_instructions.len() > 0
            && self.ip
                < self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .instructions
                    .len()
        {
            let instruction = self
                .lambda_instructions
                .last()
                .unwrap()
                .as_const_type::<VMInstructions>()
                .instructions[self.ip]
                .clone();
            self.execute_instruction(instruction, gc_system)?;
            self.ip += 1;
        }

        Ok(result)
    }
}

impl IRExecutor {
    pub fn execute_instruction(
        &mut self,
        instruction: IR,
        gc_system: &mut GCSystem,
    ) -> Result<GCRef, VMError> {
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
                        tuple_original.push(obj.clone());
                        let obj_ref = try_ref_as_vmobject(obj.clone());
                        if obj_ref.is_err() {
                            return Err(VMError::UnableToReference(obj));
                        }
                        tuple.push(obj_ref.unwrap());
                    } else {
                        return Err(VMError::InvaildInstruction(instruction.clone()));
                    }
                }
                let obj = gc_system.new_object(VMTuple::new(tuple.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                for obj in tuple_original {
                    obj.offline(); // offline the tuple because it is not stored in context
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
                value_original.offline(); // offline the tuple because it is not stored in context
                key_original.offline(); // offline the tuple because it is not stored in context
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
                    _ => return Err(VMError::InvaildInstruction(instruction.clone())),
                };
                self.stack.push(VMStackObject::VMObject(obj));
                left_original.offline(); // offline the tuple because it is not stored in context
                right_original.offline(); // offline the tuple because it is not stored in context
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
                        let ref_obj = ref_obj.unwrap();
                        if !ref_obj.isinstance::<VMBoolean>() {
                            return Err(VMError::InvaildInstruction(instruction.clone()));
                        }
                        gc_system.new_object(VMBoolean::new(!ref_obj.as_const_type::<VMBoolean>().value))
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
                original.offline(); // offline the tuple because it is not stored in context
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
                if !obj_ref.isinstance::<VMVariableWrapper>() {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                }
                let result = self.context.set_var(name, obj_ref.clone());
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
                value.offline();
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
                self.ip = ip;
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
                self.ip = (self.ip as isize + offset) as usize;
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
                    self.ip = (self.ip as isize + offset) as usize;
                }
                obj.offline(); // offline the tuple because it is not stored in context
            }
            IR::ResetStack => {
                for i in *self.context.stack_pointers.last().unwrap()..self.stack.len() {
                    let obj = self.stack[i].clone();
                    if let VMStackObject::VMObject(obj) = obj {
                        obj.offline();
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
                obj.offline(); // offline the tuple because it is not stored in context
                attr.offline(); // offline the tuple because it is not stored in context
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
                let result = try_index_of_as_vmobject(ref_obj, index_ref);
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }
                let result = result.unwrap();
                self.stack.push(VMStackObject::VMObject(result));
                obj.offline(); // offline the tuple because it is not stored in context
                index.offline(); // offline the tuple because it is not stored in context
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
            }

            IR::DebugInfo(debug_info) => {
                if self.original_code.is_some() {
                    let debug_info = debug_info.clone();
                    self.set_debug_info(debug_info);
                }
            }

            _ => return Err(VMError::InvaildInstruction(instruction.clone())),
        }
        Ok(gc_system.new_object(VMNull::new()))
    }
}
