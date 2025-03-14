use crate::vm::ir::DebugInfo;
use crate::vm::ir::IR;

use super::super::gc::gc::*;
use super::context::*;
use super::variable::*;

#[derive(Debug)]
pub enum VMError {
    InvaildInstruction(IR),
    TryEnterNotLambda(GCRef),
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
    pub fn new(original_code:  Option<String>) -> Self {
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
        match instruction {
            IR::LoadInt(value) => {
                let obj = gc_system.new_object(VMInt::new(value));
                self.stack.push(VMStackObject::VMObject(obj));
            }
            _ => return Err(VMError::InvaildInstruction(instruction.clone())),
        }
        Ok(gc_system.new_object(VMNull::new()))
    }
}
