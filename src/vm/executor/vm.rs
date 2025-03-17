use std::collections::HashMap;
use std::default;
use std::fs::File;
use std::io::Read;

use rand::seq::index;

use crate::vm::ir::DebugInfo;
use crate::vm::ir::IROperation;
use crate::vm::ir::IRPackage;
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
    FileError(String),
}

impl VMError {
    pub fn to_string(&self) -> String {
        match self {
            VMError::InvaildInstruction(instruction) => {
                format!("InvaildInstruction: {:?}", instruction)
            }
            VMError::TryEnterNotLambda(lambda) => format!(
                "TryEnterNotLambda: {}",
                try_repr_vmobject(lambda.clone()).unwrap_or(format!("{:?}", lambda))
            ),
            VMError::EmptyStack => "EmptyStack".to_string(),
            VMError::ArgumentIsNotTuple(tuple) => format!(
                "ArgumentIsNotTuple: {}",
                try_repr_vmobject(tuple.clone()).unwrap_or(format!("{:?}", tuple))
            ),
            VMError::UnableToReference(obj) => format!(
                "UnableToReference: {}",
                try_repr_vmobject(obj.clone()).unwrap_or(format!("{:?}", obj))
            ),
            VMError::NotVMObject(obj) => format!("NotVMObject: {:?}", obj),
            VMError::ContextError(err) => format!("ContextError: {:?}", err.to_string()),
            VMError::VMVariableError(err) => format!("VMVariableError: {}", err.to_string()),
            VMError::AssertFailed => "AssertFailed".to_string(),
            VMError::CannotGetSelf(obj) => format!(
                "CannotGetSelf: {}",
                try_repr_vmobject(obj.clone()).unwrap_or(format!("{:?}", obj))
            ),
            VMError::InvaildArgument(obj, msg) => format!(
                "InvaildArgument: {}, {}",
                try_repr_vmobject(obj.clone()).unwrap_or(format!("{:?}", obj)),
                msg
            ),
            VMError::FileError(msg) => format!("FileError: {}", msg),
        }
    }
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

mod native_functions {
    use std::io::Write;

    use crate::vm::{
        executor::variable::{
            try_repr_vmobject, try_value_ref_as_vmobject, VMBoolean, VMFloat, VMInt, VMNull,
            VMString, VMTuple, VMVariableError,
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
        let tuple = tuple.as_type::<VMTuple>();
        let mut result = String::new();
        for obj in tuple.values.iter() {
            let obj_ref = try_value_ref_as_vmobject(obj.clone())?;
            let repr = try_repr_vmobject(obj_ref)?;
            result.push_str(&format!("{} ", repr));
        }
        result = result.trim_end_matches(" ").to_string();
        println!("{}", result);
        let obj = gc_system.new_object(VMNull::new());
        return Ok(obj);
    }

    pub fn len(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "len function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMTuple>() {
            let inner_tuple = tuple_obj.values[0].as_type::<VMTuple>();
            let obj = gc_system.new_object(VMInt::new(inner_tuple.values.len() as i64));
            return Ok(obj);
        } else if tuple_obj.values[0].isinstance::<VMString>() {
            let inner_string = tuple_obj.values[0].as_type::<VMString>();
            let obj = gc_system.new_object(VMInt::new(inner_string.value.len() as i64));
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
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_int function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_type::<VMInt>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_type::<VMFloat>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_type::<VMString>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMInt::new(0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_type::<VMBoolean>().to_int()?;
            return Ok(gc_system.new_object(VMInt::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_int function's input should be a int".to_string(),
        ))
    }

    pub fn to_float(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_float function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_type::<VMInt>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_type::<VMFloat>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_type::<VMString>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMFloat::new(0.0)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_type::<VMBoolean>().to_float()?;
            return Ok(gc_system.new_object(VMFloat::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_float function's input should be a float".to_string(),
        ))
    }

    pub fn to_string(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_string function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_type::<VMInt>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_type::<VMFloat>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_type::<VMString>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMString::new("null".to_string())));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_type::<VMBoolean>().to_string()?;
            return Ok(gc_system.new_object(VMString::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_string function's input should be a string".to_string(),
        ))
    }

    pub fn to_bool(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "to_bool function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMInt>() {
            let data = tuple_obj.values[0].as_type::<VMInt>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMFloat>() {
            let data = tuple_obj.values[0].as_type::<VMFloat>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_type::<VMString>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        if tuple_obj.values[0].isinstance::<VMNull>() {
            return Ok(gc_system.new_object(VMBoolean::new(false)));
        }
        if tuple_obj.values[0].isinstance::<VMBoolean>() {
            let data = tuple_obj.values[0].as_type::<VMBoolean>().to_bool()?;
            return Ok(gc_system.new_object(VMBoolean::new(data)));
        }
        Err(VMVariableError::TypeError(
            tuple.clone(),
            "to_bool function's input should be a bool".to_string(),
        ))
    }

    pub fn input(tuple: GCRef, gc_system: &mut GCSystem) -> Result<GCRef, VMVariableError> {
        check_if_tuple(tuple.clone())?;
        let tuple_obj = tuple.as_type::<VMTuple>();
        if tuple_obj.values.len() != 1 {
            return Err(VMVariableError::TypeError(
                tuple.clone(),
                "input function's input should be one element".to_string(),
            ));
        }
        if tuple_obj.values[0].isinstance::<VMString>() {
            let data = tuple_obj.values[0].as_type::<VMString>().to_string()?;
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
        context: &mut Context,
        gc_system: &mut GCSystem,
    ) -> Result<(), VMError> {
        let mut built_in_functions: HashMap<String, GCRef> = HashMap::new();
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
            "bool".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::to_bool)),
        );
        built_in_functions.insert(
            "input".to_string(),
            gc_system.new_object(VMNativeFunction::new(native_functions::input)),
        );

        for (name, func) in built_in_functions.iter() {
            let result = context.let_var(name.clone(), func.clone(), true, gc_system);
            func.offline();
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
                    let repr = try_repr_vmobject(obj.clone());
                    if repr.is_ok() {
                        println!("{}: {:?}", i, repr.unwrap());
                    } else {
                        println!("{}: {:?}", i, obj);
                    }
                }
                VMStackObject::LastIP(ip, use_new_instructions) => {
                    println!("{}: LastIP: {} {}", i, ip, use_new_instructions);
                }
            }
        }
    }

    pub fn pop_and_ref(&mut self) -> Result<(GCRef, GCRef), VMError> {
        let obj = self.pop_object()?;
        let obj = match obj {
            VMStackObject::VMObject(obj) => obj,
            _ => return Err(VMError::NotVMObject(obj)),
        };
        let obj_ref =
            try_value_ref_as_vmobject(obj.clone()).map_err(|e| VMError::VMVariableError(e))?;
        Ok((obj, obj_ref))
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
        self.stack.push(VMStackObject::LastIP(
            self.ip as usize,
            use_new_instructions,
        ));
        self.context
            .new_frame(&self.stack, true, lambda.code_position, false);

        let default_args = lambda.default_args_tuple.clone();

        for v in default_args.as_type::<VMTuple>().values.iter() {
            let v_ref = try_value_ref_as_vmobject(v.clone())
                .map_err(|_| VMError::UnableToReference(v.clone()))?;

            if !v_ref.isinstance::<VMNamed>() {
                return Err(VMError::InvaildArgument(
                    v.clone(),
                    format!("Not a VMNamed in Lambda arguments: {:?}", v),
                ));
            }
            let v = v_ref.as_const_type::<VMNamed>();
            let name = v.key.clone();
            let value = v.value.clone();

            if !name.isinstance::<VMString>() {
                return Err(VMError::InvaildArgument(
                    name.clone(),
                    format!(
                        "Expected VMString in Lambda arguments {}'s key, but got {}",
                        try_repr_vmobject(v_ref.clone()).unwrap_or(format!("{:?}", v_ref)),
                        try_repr_vmobject(name.clone()).unwrap_or(format!("{:?}", name))
                    ),
                ));
            }
            let name = name.as_const_type::<VMString>();
            let name = name.value.clone();
            let value_ref = try_value_ref_as_vmobject(value.clone())
                .map_err(|_| VMError::UnableToReference(value))?;
            let result = self
                .context
                .let_var(name.clone(), value_ref.clone(), false, gc_system);
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

        if lambda.self_object.is_some() {
            let self_obj = lambda.self_object.clone().unwrap();
            let self_obj_ref = try_value_ref_as_vmobject(self_obj.clone());
            if self_obj_ref.is_err() {
                return Err(VMError::UnableToReference(self_obj));
            }
            let self_obj_ref = self_obj_ref.unwrap();
            let result =
                self.context
                    .let_var("self".to_string(), self_obj_ref.clone(), false, gc_system);
            if result.is_err() {
                return Err(VMError::ContextError(result.unwrap_err()));
            }
        }

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

            //println!("# {} {}: {:?}", gc_system.count(), self.ip, instruction); // debug
            //self.context.debug_print_all_vars();
            //self.debug_output_stack();
            self.execute_instruction(instruction.clone(), gc_system)?;

            //self.debug_output_stack(); // debug
            //println!("");

            //gc_system.collect(); // debug
            //println!("GC Count: {}", gc_system.count()); // debug
            //gc_system.print_reference_graph(); // debug
            self.ip += 1;
        }
        let result = self.stack.pop();
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
    pub fn is_variable(&self, object: &GCRef) -> bool {
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
                let (default_args_tuple, default_args_tuple_ref) = self.pop_and_ref()?;
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
                self.offline_if_not_variable(&default_args_tuple);
            }
            IR::BuildTuple(size) => {
                let mut tuple = Vec::new();
                let mut tuple_refs: Vec<GCRef> = Vec::new();
                for _ in 0..*size {
                    let (obj, obj_ref) = self.pop_and_ref()?;
                    tuple.push(obj);
                    tuple_refs.push(obj_ref);
                }
                let obj = gc_system.new_object(VMTuple::new(tuple_refs.clone()));

                let built_tuple = obj.as_type::<VMTuple>();
                for val in &built_tuple.values {
                    if val.isinstance::<VMNamed>()
                        && val
                            .as_const_type::<VMNamed>()
                            .value
                            .isinstance::<VMLambda>()
                    {
                        let lambda = val.as_const_type::<VMNamed>().value.as_type::<VMLambda>();
                        lambda.set_self_object(obj.clone());
                    }
                }

                self.stack.push(VMStackObject::VMObject(obj));
                for obj in tuple {
                    self.offline_if_not_variable(&obj);
                }
            }

            IR::BuildKeyValue => {
                let (value, value_ref) = self.pop_and_ref()?;
                let (key, key_ref) = self.pop_and_ref()?;
                let obj = gc_system.new_object(VMKeyVal::new(key_ref.clone(), value_ref.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&key);
                self.offline_if_not_variable(&value);
            }

            IR::BuildNamed => {
                let (value, value_ref) = self.pop_and_ref()?;
                let (key, key_ref) = self.pop_and_ref()?;
                let obj = gc_system.new_object(VMNamed::new(key_ref.clone(), value_ref.clone()));
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&key);
                self.offline_if_not_variable(&value);
            }

            IR::BinaryOp(operation) => {
                let (right_original, right) = self.pop_and_ref()?;
                let (left_original, left) = self.pop_and_ref()?;

                let obj = match operation {
                    IROperation::Equal => {
                        gc_system.new_object(VMBoolean::new(try_eq_as_vmobject(left, right)))
                    }
                    IROperation::NotEqual => {
                        gc_system.new_object(VMBoolean::new(!try_eq_as_vmobject(left, right)))
                    }
                    IROperation::Greater => {
                        let result = try_greater_than_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(result))
                    }
                    IROperation::Less => {
                        let result = try_less_than_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(result))
                    }
                    IROperation::GreaterEqual => {
                        let result = try_less_than_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(!result))
                    }
                    IROperation::LessEqual => {
                        let result = try_greater_than_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(!result))
                    }

                    IROperation::Add => try_add_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::Subtract => try_sub_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::Multiply => try_mul_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::Divide => try_div_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::Modulus => try_mod_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::BitwiseAnd => try_bitwise_and_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::BitwiseOr => try_bitwise_or_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::BitwiseXor => try_bitwise_xor_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::ShiftLeft => try_shift_left_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::ShiftRight => try_shift_right_as_vmobject(left, right, gc_system)
                        .map_err(|e| VMError::VMVariableError(e))?,

                    IROperation::And => {
                        let result = try_and_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(result))
                    }

                    IROperation::Or => {
                        let result = try_or_as_vmobject(left, right)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(result))
                    }
                    _ => return Err(VMError::InvaildInstruction(instruction)),
                };
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&left_original);
                self.offline_if_not_variable(&right_original);
            }

            IR::UnaryOp(operation) => {
                let (original, ref_obj) = self.pop_and_ref()?;
                let obj = match operation {
                    IROperation::Not => {
                        let result = try_not_as_vmobject(ref_obj)
                            .map_err(|e| VMError::VMVariableError(e))?;
                        gc_system.new_object(VMBoolean::new(result))
                    }
                    IROperation::Subtract => {
                        let ref_obj = ref_obj;
                        if ref_obj.isinstance::<VMInt>() {
                            let value = ref_obj.as_const_type::<VMInt>().value;
                            gc_system.new_object(VMInt::new(-value))
                        } else if ref_obj.isinstance::<VMFloat>() {
                            let value = ref_obj.as_const_type::<VMFloat>().value;
                            gc_system.new_object(VMFloat::new(-value))
                        } else {
                            return Err(VMError::InvaildInstruction(instruction));
                        }
                    }
                    _ => return Err(VMError::InvaildInstruction(instruction)),
                };
                self.stack.push(VMStackObject::VMObject(obj));
                self.offline_if_not_variable(&original);
            }

            IR::Let(name) => {
                let (obj, obj_ref) = self.pop_and_ref()?;
                let result = self.context.let_var(name.clone(), obj_ref, true, gc_system);
                if result.is_err() {
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
                self.stack.push(VMStackObject::VMObject(obj));
            }

            IR::Get(name) => {
                let obj = self
                    .context
                    .get_var(name)
                    .map_err(|e| VMError::ContextError(e))?;
                self.stack.push(VMStackObject::VMObject(obj));
            }

            IR::Set => {
                let (value, value_ref) = self.pop_and_ref()?;
                let reference = self.pop_object()?;
                let reference = match reference {
                    VMStackObject::VMObject(reference) => reference,
                    _ => return Err(VMError::NotVMObject(reference)),
                };
                let result = try_assign_as_vmobject(reference, value_ref)
                    .map_err(|e| VMError::VMVariableError(e))?;
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&value);
            }

            IR::Return => {
                if self.stack.len() < *self.context.stack_pointers.last().unwrap() {
                    return Err(VMError::EmptyStack);
                }
                let (obj, obj_ref) = self.pop_and_ref()?;
                self.stack
                    .truncate(*self.context.stack_pointers.last().unwrap());
                let ip_info = self.stack.pop().unwrap();
                let VMStackObject::LastIP(ip, use_new_instructions) = ip_info else {
                    return Err(VMError::InvaildInstruction(instruction));
                };
                self.ip = ip as isize;
                if use_new_instructions {
                    self.lambda_instructions.pop();
                }
                let result = self.context.pop_frame(&mut self.stack, true);
                if result.is_err() {
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
                self.stack.push(VMStackObject::VMObject(obj_ref));
                self.offline_if_not_variable(&obj);
            }

            IR::NewFrame => {
                self.context.new_frame(&mut self.stack, false, 0, false);
            }
            IR::PopFrame => {
                let obj = self.pop_object()?;
                let obj = match obj {
                    VMStackObject::VMObject(obj) => obj,
                    _ => return Err(VMError::NotVMObject(obj)),
                };

                let result = self.context.pop_frame(&mut self.stack, false);
                if result.is_err() {
                    return Err(VMError::ContextError(result.unwrap_err()));
                }
                self.stack.push(VMStackObject::VMObject(obj));
            }
            IR::JumpOffset(offset) => {
                self.ip += offset;
            }
            IR::JumpIfFalseOffset(offset) => {
                let (obj, ref_obj) = self.pop_and_ref()?;
                if !ref_obj.isinstance::<VMBoolean>() {
                    return Err(VMError::VMVariableError(VMVariableError::TypeError(
                        ref_obj.clone(),
                        "JumpIfFalseOffset: Not a boolean".to_string(),
                    )));
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
                self.stack
                    .truncate(*self.context.stack_pointers.last().unwrap());
            }
            IR::GetAttr => {
                let (attr, attr_ref) = self.pop_and_ref()?;
                let (obj, ref_obj) = self.pop_and_ref()?;

                let result = try_get_attr_as_vmobject(ref_obj, attr_ref)
                    .map_err(|e| VMError::VMVariableError(e))?;
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
                self.offline_if_not_variable(&attr);
            }

            IR::IndexOf => {
                let (index, index_ref) = self.pop_and_ref()?;
                let (obj, ref_obj) = self.pop_and_ref()?;
                let result = try_index_of_as_vmobject(ref_obj, index_ref, gc_system)
                    .map_err(|e| VMError::VMVariableError(e))?;
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
                self.offline_if_not_variable(&index);
            }

            IR::KeyOf => {
                let (obj, ref_obj) = self.pop_and_ref()?;
                let result =
                    try_key_of_as_vmobject(ref_obj).map_err(|e| VMError::VMVariableError(e))?;
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
            }

            IR::ValueOf => {
                let (obj, ref_obj) = self.pop_and_ref()?;
                let result =
                    try_value_of_as_vmobject(ref_obj).map_err(|e| VMError::VMVariableError(e))?;
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&obj);
            }

            IR::Assert => {
                let (obj, ref_obj) = self.pop_and_ref()?;
                if !ref_obj.isinstance::<VMBoolean>() {
                    return Err(VMError::InvaildInstruction(instruction.clone()));
                }
                if !ref_obj.as_const_type::<VMBoolean>().value {
                    return Err(VMError::AssertFailed);
                }
                self.offline_if_not_variable(&obj);
            }

            IR::SelfOf => {
                let (obj, ref_obj) = self.pop_and_ref()?;

                if !ref_obj.isinstance::<VMLambda>() {
                    return Err(VMError::CannotGetSelf(obj));
                }
                let lambda = ref_obj.as_const_type::<VMLambda>();
                let self_obj = lambda.self_object.clone();
                match self_obj {
                    Some(self_obj) => {
                        let self_obj_ref = try_value_ref_as_vmobject(self_obj.clone());
                        if self_obj_ref.is_err() {
                            return Err(VMError::UnableToReference(self_obj));
                        }
                        let self_obj_ref = self_obj_ref.unwrap();
                        self.stack.push(VMStackObject::VMObject(self_obj_ref));
                    }
                    None => {
                        self.stack
                            .push(VMStackObject::VMObject(gc_system.new_object(VMNull::new())));
                    }
                }
                self.offline_if_not_variable(&obj);
            }

            IR::CopyValue => {
                let (obj, ref_obj) = self.pop_and_ref()?;

                let result = try_copy_as_vmobject(ref_obj.clone(), gc_system).map_err(|_| {
                    VMError::VMVariableError(VMVariableError::TypeError(
                        ref_obj.clone(),
                        "Not a copyable object".to_string(),
                    ))
                })?;
                self.stack.push(VMStackObject::VMObject(result));

                self.offline_if_not_variable(&obj);
            }

            IR::RefValue => return Err(VMError::InvaildInstruction(instruction.clone())),

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
                let (arg_tuple, arg_tuple_ref) = self.pop_and_ref()?;
                let (lambda, lambda_ref) = self.pop_and_ref()?;

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

                let lambda_obj = lambda_ref.as_const_type::<VMLambda>();

                let signature = lambda_obj.signature.clone();
                let default_args = lambda_obj.default_args_tuple.clone();

                let result = default_args
                    .as_type::<VMTuple>()
                    .assign_members(arg_tuple_ref.clone());
                if result.is_err() {
                    return Err(VMError::VMVariableError(result.unwrap_err()));
                }

                self.enter_lambda(lambda_ref.clone(), gc_system)?;

                let func_ips = &self
                    .lambda_instructions
                    .last()
                    .unwrap()
                    .as_const_type::<VMInstructions>()
                    .func_ips;
                let ip = func_ips.get(&signature).unwrap().clone() as isize;
                self.ip = ip - 1;

                self.offline_if_not_variable(&arg_tuple);
                self.offline_if_not_variable(&lambda);
            }

            IR::Wrap => {
                let (obj, ref_obj) = self.pop_and_ref()?;
                let wrapped = VMWrapper::new(ref_obj);
                let wrapped = gc_system.new_object(wrapped);
                self.stack.push(VMStackObject::VMObject(wrapped));
                self.offline_if_not_variable(&obj);
            }

            IR::In => {
                let (container, container_ref) = self.pop_and_ref()?;
                let (obj, ref_obj) = self.pop_and_ref()?;

                let result = try_contains_as_vmobject(container_ref, ref_obj).map_err(|_| {
                    VMError::VMVariableError(VMVariableError::TypeError(
                        container.clone(),
                        "Not a container".to_string(),
                    ))
                })?;
                self.stack.push(VMStackObject::VMObject(
                    gc_system.new_object(VMBoolean::new(result)),
                ));
                self.offline_if_not_variable(&obj);
                self.offline_if_not_variable(&container);
            }

            IR::BuildRange => {
                let (end, end_ref) = self.pop_and_ref()?;
                let (start, start_ref) = self.pop_and_ref()?;

                if !start_ref.isinstance::<VMInt>() {
                    return Err(VMError::InvaildArgument(
                        start_ref.clone(),
                        "Start of range is not a VMInt".to_string(),
                    ));
                }
                if !end_ref.isinstance::<VMInt>() {
                    return Err(VMError::InvaildArgument(
                        end_ref.clone(),
                        "End of range is not a VMInt".to_string(),
                    ));
                }
                let start_ref = start_ref.as_const_type::<VMInt>();
                let end_ref = end_ref.as_const_type::<VMInt>();

                let result = gc_system.new_object(VMRange::new(start_ref.value, end_ref.value));
                self.stack.push(VMStackObject::VMObject(result));
                self.offline_if_not_variable(&start);
                self.offline_if_not_variable(&end);
            }
            IR::TypeOf => {
                let (obj, ref_obj) = self.pop_and_ref()?;

                if ref_obj.isinstance::<VMInt>() {
                    let result = gc_system.new_object(VMString::new("int".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMFloat>() {
                    let result = gc_system.new_object(VMString::new("float".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMString>() {
                    let result = gc_system.new_object(VMString::new("string".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMBoolean>() {
                    let result = gc_system.new_object(VMString::new("bool".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMTuple>() {
                    let result = gc_system.new_object(VMString::new("tuple".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMLambda>() {
                    let result = gc_system.new_object(VMString::new("lambda".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMNull>() {
                    let result = gc_system.new_object(VMString::new("null".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMKeyVal>() {
                    let result = gc_system.new_object(VMString::new("keyval".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMNamed>() {
                    let result = gc_system.new_object(VMString::new("named".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMRange>() {
                    let result = gc_system.new_object(VMString::new("range".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMWrapper>() {
                    let result = gc_system.new_object(VMString::new("wrapper".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else if ref_obj.isinstance::<VMBoolean>() {
                    let result = gc_system.new_object(VMString::new("bool".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                } else {
                    let result = gc_system.new_object(VMString::new("".to_string()));
                    self.stack.push(VMStackObject::VMObject(result));
                }
                self.offline_if_not_variable(&obj);
            }

            IR::Import(code_position) => {
                let (path_arg_named, path_arg_named_ref) = self.pop_and_ref()?;

                if !path_arg_named_ref.isinstance::<VMNamed>() {
                    return Err(VMError::InvaildArgument(
                        path_arg_named_ref.clone(),
                        format!(
                            "Import requires VMNamed but got {:?}",
                            try_repr_vmobject(path_arg_named_ref.clone())
                        ),
                    ));
                }

                let path_arg_named_ref = path_arg_named_ref.as_const_type::<VMNamed>();
                let path = path_arg_named_ref.key.clone();
                let path_ref = try_value_ref_as_vmobject(path.clone());
                if path_ref.is_err() {
                    return Err(VMError::UnableToReference(path));
                }
                let path_ref = path_ref.unwrap();
                if !path_ref.isinstance::<VMString>() {
                    return Err(VMError::InvaildArgument(
                        path_ref.clone(),
                        format!(
                            "Import requires VMString but got {:?}",
                            try_repr_vmobject(path_ref.clone())
                        ),
                    ));
                }
                let path_ref = path_ref.as_const_type::<VMString>();

                let arg_tuple = path_arg_named_ref.value.clone();
                let arg_tuple_ref = try_value_ref_as_vmobject(arg_tuple.clone())
                    .map_err(|_| VMError::UnableToReference(path_arg_named_ref.value.clone()))?;
                if !arg_tuple_ref.isinstance::<VMTuple>() {
                    return Err(VMError::InvaildArgument(
                        arg_tuple_ref.clone(),
                        format!(
                            "Import as VMLambda requires VMTuple but got {:?}",
                            try_repr_vmobject(arg_tuple_ref.clone())
                        ),
                    ));
                }

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

                let lambda = VMLambda::new(
                    *code_position,
                    "__main__".to_string(),
                    arg_tuple_ref,
                    None,
                    vm_instructions.clone(),
                );

                let lambda = gc_system.new_object(lambda);

                self.stack.push(VMStackObject::VMObject(lambda));
                self.offline_if_not_variable(&path_arg_named);
                vm_instructions.offline();
            }

            _ => return Err(VMError::InvaildInstruction(instruction.clone())),
        }

        Ok(())
    }
}
