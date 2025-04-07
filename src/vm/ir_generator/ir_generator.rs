use super::super::ir::{Functions, IROperation};
use crate::{
    parser::ast::{ASTNode, ASTNodeModifier, ASTNodeOperation, ASTNodeType},
    vm::ir::{DebugInfo, IR},
};
use base64::{self, Engine};
#[derive(Debug)]
enum Scope {
    Frame,
    Loop(String, String), // loop head label, loop end label
}

#[derive(Debug, Clone)]
pub struct NameSpace {
    path: Vec<String>,
}

impl NameSpace {
    pub fn new(name: String, parent: Option<&NameSpace>) -> Self {
        let mut path = Vec::new();
        if let Some(parent) = parent {
            path.extend(parent.path.clone());
        }
        path.push(name);
        NameSpace { path }
    }

    pub fn get_full_name(&self) -> String {
        self.path.join("::")
    }
}

#[derive(Debug)]
pub struct LabelGenerator {
    label_counter: usize,
    namespace: Rc<NameSpace>,
    type_name: String,
}

impl LabelGenerator {
    pub fn new(namespace: Rc<NameSpace>, type_name: String) -> Self {
        LabelGenerator {
            label_counter: 0,
            namespace,
            type_name,
        }
    }

    pub fn new_label(&mut self) -> (String, String) {
        let full_label = format!(
            "{}::{}_{}",
            self.namespace.get_full_name(),
            self.type_name,
            self.label_counter
        );
        let label = format!("{}_{}", self.type_name, self.label_counter);
        self.label_counter += 1;
        (full_label, label)
    }

    pub fn reset(&mut self) {
        self.label_counter = 0;
    }
}

#[derive(Debug)]
pub struct IRGenerator<'t> {
    namespace: Rc<NameSpace>,
    functions: &'t mut Functions,
    scope_stack: Vec<Scope>,
    label_generator: LabelGenerator,
    function_signature_generator: LabelGenerator,
}

#[derive(Debug)]
pub enum IRGeneratorError {
    InvalidASTNodeType(ASTNodeType),
    InvalidScope,
    InvalidLabel,
    InvalidFunctionSignature,
}

use std::rc::Rc;

impl<'t> IRGenerator<'t> {
    pub fn new(functions: &'t mut Functions, namespace: NameSpace) -> Self {
        let namespace_rc = Rc::new(namespace);
        let label_generator = LabelGenerator::new(Rc::clone(&namespace_rc), "label".to_string());
        let function_signature_generator =
            LabelGenerator::new(Rc::clone(&namespace_rc), "function".to_string());
        IRGenerator {
            namespace: namespace_rc,
            functions,
            scope_stack: Vec::new(),
            label_generator,
            function_signature_generator,
        }
    }

    fn new_label(&mut self) -> (String, String) {
        self.label_generator.new_label()
    }

    fn new_function_signature(&mut self) -> (String, String) {
        self.function_signature_generator.new_label()
    }

    fn generate_debug_info(&mut self, ast_node: &ASTNode) -> DebugInfo {
        DebugInfo {
            code_position: match ast_node.token {
                Some(token) => token.position,
                None => 0,
            },
        }
    }

    pub fn generate_without_redirect(
        &mut self,
        ast_node: &ASTNode,
    ) -> Result<Vec<(DebugInfo, IR)>, IRGeneratorError> {
        match &ast_node.node_type {
            ASTNodeType::Body => {
                let mut instructions = Vec::new();
                instructions.push((self.generate_debug_info(ast_node), IR::NewFrame));
                self.scope_stack.push(Scope::Frame);
                let children_len = ast_node.children.len();
                for i in 0..children_len {
                    let child = &ast_node.children[i];
                    let child_instructions = self.generate_without_redirect(child)?;
                    instructions.extend(child_instructions);
                }
                self.scope_stack.pop();
                instructions.push((self.generate_debug_info(ast_node), IR::PopFrame));
                Ok(instructions)
            }
            ASTNodeType::LambdaDef(is_dyn) => {
                let mut instructions = Vec::new();
                let args = &ast_node.children[0];
                let args_instructions = self.generate_without_redirect(args)?;

                if *is_dyn {
                    let expr_instructions =
                        self.generate_without_redirect(&ast_node.children[1])?;
                    instructions.extend(args_instructions);
                    instructions.extend(expr_instructions);

                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::LoadLambda("__main__".to_string(), ast_node.token.unwrap().position),
                    ));
                } else {
                    let (full_signature, signature) = self.new_function_signature();

                    let mut generator = IRGenerator::new(
                        self.functions,
                        NameSpace::new(signature.clone(), Some(&self.namespace)),
                    );

                    let mut body_instructions = generator.generate(&ast_node.children[1])?; // body, compute redirect jump directly
                    body_instructions.push((self.generate_debug_info(ast_node), IR::Return));

                    self.functions
                        .append(full_signature.clone(), body_instructions);

                    instructions.extend(args_instructions);
                    instructions.push((self.generate_debug_info(ast_node), IR::ForkInstruction));
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::LoadLambda(full_signature, ast_node.token.unwrap().position),
                    ));
                }
                Ok(instructions)
            }
            ASTNodeType::Assign => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Set));
                Ok(instructions)
            }
            ASTNodeType::Variable(var_name) => {
                let mut instructions = Vec::new();
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::Get(var_name.clone()),
                ));
                Ok(instructions)
            }
            ASTNodeType::Let(var_name) => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::Let(var_name.clone()),
                ));
                Ok(instructions)
            }
            ASTNodeType::LambdaCall => {
                let mut instructions = Vec::new();
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                instructions.push((self.generate_debug_info(ast_node), IR::CallLambda));
                Ok(instructions)
            }
            ASTNodeType::AsyncLambdaCall => {
                let mut instructions = Vec::new();
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                instructions.push((self.generate_debug_info(ast_node), IR::AsyncCallLambda));
                Ok(instructions)
            }
            ASTNodeType::Operation(opeartion) => {
                let mut instructions = Vec::new();
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                match opeartion {
                    ASTNodeOperation::Add => {
                        if ast_node.children.len() == 1 {
                            instructions.push((
                                self.generate_debug_info(ast_node),
                                IR::UnaryOp(IROperation::Add),
                            ));
                        } else {
                            instructions.push((
                                self.generate_debug_info(ast_node),
                                IR::BinaryOp(IROperation::Add),
                            ));
                        }
                    }
                    ASTNodeOperation::Subtract => {
                        if ast_node.children.len() == 1 {
                            instructions.push((
                                self.generate_debug_info(ast_node),
                                IR::UnaryOp(IROperation::Subtract),
                            ));
                        } else {
                            instructions.push((
                                self.generate_debug_info(ast_node),
                                IR::BinaryOp(IROperation::Subtract),
                            ));
                        }
                    }
                    ASTNodeOperation::Multiply => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Multiply),
                        ));
                    }
                    ASTNodeOperation::Divide => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Divide),
                        ));
                    }
                    ASTNodeOperation::Modulus => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Modulus),
                        ));
                    }
                    ASTNodeOperation::And => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::And),
                        ));
                    }
                    ASTNodeOperation::Or => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Or),
                        ));
                    }
                    ASTNodeOperation::Xor => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Xor),
                        ));
                    }
                    ASTNodeOperation::Not => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::UnaryOp(IROperation::Not),
                        ));
                    }
                    ASTNodeOperation::Equal => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Equal),
                        ));
                    }
                    ASTNodeOperation::NotEqual => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::NotEqual),
                        ));
                    }
                    ASTNodeOperation::Greater => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Greater),
                        ));
                    }
                    ASTNodeOperation::Less => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Less),
                        ));
                    }
                    ASTNodeOperation::GreaterEqual => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::GreaterEqual),
                        ));
                    }
                    ASTNodeOperation::LessEqual => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::LessEqual),
                        ));
                    }
                    ASTNodeOperation::ShiftLeft => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::ShiftLeft),
                        ));
                    }
                    ASTNodeOperation::ShiftRight => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::ShiftRight),
                        ));
                    }
                    ASTNodeOperation::Power => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::Power),
                        ));
                    }
                    ASTNodeOperation::LeftShift => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::ShiftLeft),
                        ));
                    }
                    ASTNodeOperation::RightShift => {
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::BinaryOp(IROperation::ShiftRight),
                        ));
                    }
                }
                Ok(instructions)
            }
            ASTNodeType::IndexOf => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::IndexOf));
                Ok(instructions)
            }
            ASTNodeType::GetAttr => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::GetAttr));
                Ok(instructions)
            }
            ASTNodeType::Return => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Return));
                Ok(instructions)
            }
            ASTNodeType::Raise => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Raise));
                Ok(instructions)
            }
            ASTNodeType::Yield => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Emit));
                Ok(instructions)
            }
            ASTNodeType::Number(number_str) => {
                // check if number_str is float or int
                let mut instructions = Vec::new();
                if let Ok(number) = number_str.parse::<i64>() {
                    instructions.push((self.generate_debug_info(ast_node), IR::LoadInt(number)));
                } else if let Ok(number) = number_str.parse::<f64>() {
                    instructions.push((self.generate_debug_info(ast_node), IR::LoadFloat(number)));
                } else {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
                Ok(instructions)
            }
            ASTNodeType::Tuple => {
                let mut instructions = Vec::new();
                let mut tuple_size = 0;
                for child in &ast_node.children {
                    if child.node_type == ASTNodeType::None {
                        continue;
                    }
                    instructions.extend(self.generate_without_redirect(child)?);
                    tuple_size += 1;
                }
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::BuildTuple(tuple_size),
                ));
                Ok(instructions)
            }
            ASTNodeType::KeyValue => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::BuildKeyValue));
                Ok(instructions)
            }
            ASTNodeType::NamedTo => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::BuildNamed));
                Ok(instructions)
            }
            ASTNodeType::String(str) => {
                let mut instructions = Vec::new();
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::LoadString(str.clone()),
                ));
                Ok(instructions)
            }
            ASTNodeType::Expressions => {
                let mut instructions = Vec::new();
                for child in &ast_node.children {
                    instructions.push((self.generate_debug_info(child), IR::ResetStack));
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                Ok(instructions)
            }
            ASTNodeType::Null => {
                let mut instructions = Vec::new();
                instructions.push((self.generate_debug_info(ast_node), IR::LoadNull));
                Ok(instructions)
            }
            ASTNodeType::None => {
                let mut instructions = Vec::new();
                instructions.push((self.generate_debug_info(ast_node), IR::LoadNull));
                Ok(instructions)
            }
            ASTNodeType::Boolean(bool_str) => {
                let mut instructions = Vec::new();
                if bool_str == "true" {
                    instructions.push((self.generate_debug_info(ast_node), IR::LoadBool(true)));
                } else if bool_str == "false" {
                    instructions.push((self.generate_debug_info(ast_node), IR::LoadBool(false)));
                } else {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
                Ok(instructions)
            }
            ASTNodeType::If => match ast_node.children.len() {
                0..=1 => Err(IRGeneratorError::InvalidASTNodeType(
                    ast_node.node_type.clone(),
                )),
                2 => {
                    let mut instructions = Vec::new();
                    instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                    let body_instructions =
                        self.generate_without_redirect(&ast_node.children[1])?;
                    let (if_label, _) = self.new_label();
                    let (else_label, _) = self.new_label();
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectJumpIfFalse(if_label.clone()),
                    ));
                    instructions.extend(body_instructions);
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectJump(else_label.clone()),
                    ));
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectLabel(if_label.clone()),
                    ));
                    instructions.push((self.generate_debug_info(ast_node), IR::LoadNull));
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectLabel(else_label.clone()),
                    ));
                    Ok(instructions)
                }
                3 => {
                    let mut instructions = Vec::new();
                    instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                    let body_instructions =
                        self.generate_without_redirect(&ast_node.children[1])?;
                    let else_body_instructions =
                        self.generate_without_redirect(&ast_node.children[2])?;
                    let (if_label, _) = self.new_label();
                    let (else_label, _) = self.new_label();
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectJumpIfFalse(if_label.clone()),
                    ));
                    instructions.extend(body_instructions);
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectJump(else_label.clone()),
                    ));
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectLabel(if_label.clone()),
                    ));
                    instructions.extend(else_body_instructions);
                    instructions.push((
                        self.generate_debug_info(ast_node),
                        IR::RedirectLabel(else_label.clone()),
                    ));
                    Ok(instructions)
                }
                _ => Err(IRGeneratorError::InvalidASTNodeType(
                    ast_node.node_type.clone(),
                )),
            },
            ASTNodeType::Break => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                let mut frames_to_pop = 0;
                let mut found_loop = false;
                let mut loop_label = None;
                for scope in self.scope_stack.iter().rev() {
                    match scope {
                        Scope::Frame => {
                            frames_to_pop += 1;
                        }
                        Scope::Loop(_, end_label) => {
                            found_loop = true;
                            loop_label = Some(end_label.clone());
                            break;
                        }
                    }
                }
                if !found_loop {
                    return Err(IRGeneratorError::InvalidScope);
                }

                for _ in 0..frames_to_pop {
                    instructions.push((self.generate_debug_info(ast_node), IR::PopFrame));
                }
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectJump(loop_label.unwrap()),
                ));

                Ok(instructions)
            }
            ASTNodeType::Continue => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                let mut found_loop = false;
                let mut loop_label = None;
                for scope in self.scope_stack.iter().rev() {
                    match scope {
                        Scope::Frame => {}
                        Scope::Loop(head_label, _) => {
                            found_loop = true;
                            loop_label = Some(head_label.clone());
                            break;
                        }
                    }
                }
                if !found_loop {
                    return Err(IRGeneratorError::InvalidScope);
                }

                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectJump(loop_label.unwrap()),
                ));
                Ok(instructions)
            }
            ASTNodeType::While => {
                let mut instructions = Vec::new();
                let (head_label, _) = self.new_label();
                let (med_label, _) = self.new_label();
                let (end_label, _) = self.new_label();
                self.scope_stack
                    .push(Scope::Loop(head_label.clone(), end_label.clone()));
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectLabel(head_label.clone()),
                ));
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectJumpIfFalse(med_label.clone()),
                ));
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectJump(head_label.clone()),
                ));
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectLabel(med_label.clone()),
                ));
                instructions.push((self.generate_debug_info(ast_node), IR::LoadNull));
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectLabel(end_label.clone()),
                ));
                self.scope_stack.pop();
                Ok(instructions)
            }
            ASTNodeType::Modifier(modifier) => {
                let mut instructions = Vec::new();
                match modifier {
                    ASTNodeModifier::SelfOf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::SelfOf));
                    }
                    ASTNodeModifier::KeyOf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::KeyOf));
                    }
                    ASTNodeModifier::ValueOf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::ValueOf));
                    }
                    ASTNodeModifier::Ref => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::RefValue));
                    }
                    ASTNodeModifier::Deref => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::DerefValue));
                    }
                    ASTNodeModifier::Assert => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::Assert));
                    }
                    ASTNodeModifier::Copy => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::CopyValue));
                    }
                    ASTNodeModifier::DeepCopy => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::DeepCopyValue));
                    }
                    ASTNodeModifier::Import => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::Import));
                    }
                    ASTNodeModifier::TypeOf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::TypeOf));
                    }
                    ASTNodeModifier::Wrap => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::Wrap));
                    }
                    ASTNodeModifier::Await => {
                        let (label, _) = self.new_label();
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::RedirectLabel(label.clone()),
                        ));
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::IsFinished));
                        instructions.push((
                            self.generate_debug_info(ast_node),
                            IR::RedirectJumpIfFalse(label),
                        ));
                        instructions.push((self.generate_debug_info(ast_node), IR::LoadNull));
                    }
                    ASTNodeModifier::Wipe => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::WipeAlias));
                    }
                    ASTNodeModifier::AliasOf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::AliasOf));
                    }
                    ASTNodeModifier::BindSelf => {
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::BindSelf));
                    }
                    ASTNodeModifier::Collect => {
                        let (label1, _) = self.new_label();
                        let (label2, _) = self.new_label();
                        let (label3, _) = self.new_label();
                        let (label4, _) = self.new_label();
                        instructions.push((self.generate_debug_info(ast_node), IR::BuildTuple(0)));
                        instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                        instructions.push((self.generate_debug_info(ast_node), IR::ResetIter));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label4.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectNextOrJump(label1.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::ForkStackObjectRef(1)));
                        instructions.push((self.generate_debug_info(ast_node), IR::ValueOf));
                        instructions.push((self.generate_debug_info(ast_node), IR::ForkStackObjectRef(1)));
                        instructions.push((self.generate_debug_info(ast_node), IR::BuildTuple(1)));
                        instructions.push((self.generate_debug_info(ast_node), IR::CallLambda));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectJumpIfFalse(label2.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::PushValueIntoTuple(2)));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectJump(label3.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label2.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                        instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label3.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectJump(label4.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label1.clone())));
                        instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                    }
                }
                Ok(instructions)
            }
            ASTNodeType::Range => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::BuildRange));
                Ok(instructions)
            }
            ASTNodeType::In => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::In));
                let (label_1, _) = self.new_label();
                let (label_2, _) = self.new_label();
                instructions.push((self.generate_debug_info(ast_node), IR::RedirectJumpIfFalse(label_1.clone())));
                instructions.push((self.generate_debug_info(ast_node), IR::ValueOf));
                instructions.push((self.generate_debug_info(ast_node), IR::Swap(0, 1)));
                instructions.push((self.generate_debug_info(ast_node), IR::BuildTuple(1)));
                instructions.push((self.generate_debug_info(ast_node), IR::CallLambda));
                instructions.push((self.generate_debug_info(ast_node), IR::RedirectJump(label_2.clone())));
                instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label_1)));
                instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                instructions.push((self.generate_debug_info(ast_node), IR::Pop));
                instructions.push((self.generate_debug_info(ast_node), IR::LoadBool(false)));
                instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label_2)));
                Ok(instructions)
            }
            ASTNodeType::Alias(alias) => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((self.generate_debug_info(ast_node), IR::Alias(alias.clone())));
                Ok(instructions)
            }
            ASTNodeType::Base64(base64_str) => {
                let mut instructions = Vec::new();
                // decode base64 string to bytes using the recommended Engine approach
                let decoded_bytes = base64::engine::general_purpose::STANDARD
                    .decode(base64_str)
                    .map_err(|_| {
                        IRGeneratorError::InvalidASTNodeType(ast_node.node_type.clone())
                    })?;
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::LoadBytes(decoded_bytes),
                ));
                Ok(instructions)
            }
            ASTNodeType::Boundary => {
                let mut instructions = Vec::new();
                let (label, _) = self.new_label();
                instructions.push((
                    self.generate_debug_info(ast_node),
                    IR::RedirectNewBoundaryFrame(label.clone()),
                ));
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push((self.generate_debug_info(ast_node), IR::PopBoundaryFrame));
                instructions.push((self.generate_debug_info(ast_node), IR::RedirectLabel(label)));
                Ok(instructions)
            }
            ASTNodeType::AssumeTuple => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                Ok(instructions)
            }
            ASTNodeType::Set => {
                let mut instructions = Vec::new();
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push((self.generate_debug_info(ast_node), IR::BuildSet));
                Ok(instructions)
            }
        }
    }
    /// 重定向所有跳转指令，将RedirectJump和RedirectJumpIfFalse转换为JumpOffset和JumpIfFalse
    ///
    /// # Arguments
    ///
    /// * `irs` - IR指令列表
    ///
    /// # Returns
    ///
    /// 处理后的IR指令列表
    fn redirect_jump(
        &self,
        irs: Vec<(DebugInfo, IR)>,
    ) -> Result<Vec<(DebugInfo, IR)>, IRGeneratorError> {
        let mut reduced_irs = Vec::new();
        let mut label_map = std::collections::HashMap::new();

        // 首先收集所有标签的位置
        for ir in irs.iter() {
            if let (_, IR::RedirectLabel(label)) = ir {
                label_map.insert(label.clone(), reduced_irs.len());
            } else {
                reduced_irs.push(ir.clone());
            }
        }

        // 转换所有跳转指令
        for i in 0..reduced_irs.len() {
            match &reduced_irs[i] {
                (debug_info, IR::RedirectJump(label)) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize - 1;
                        reduced_irs[i] = (debug_info.clone(), IR::JumpOffset(offset));
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                }
                (debug_info, IR::RedirectJumpIfFalse(label)) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize - 1;
                        reduced_irs[i] = (debug_info.clone(), IR::JumpIfFalseOffset(offset));
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                }
                (debug_info, IR::RedirectNewBoundaryFrame(label)) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize - 1;
                        reduced_irs[i] = (debug_info.clone(), IR::NewBoundaryFrame(offset));
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                }
                (debug_info, IR::RedirectNextOrJump(label)) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize - 1;
                        reduced_irs[i] = (debug_info.clone(), IR::NextOrJump(offset));
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                }
                _ => {}
            }
        }

        Ok(reduced_irs)
    }

    /// 生成并优化IR指令
    ///
    /// # Arguments
    ///
    /// * `ast_node` - AST节点
    ///
    /// # Returns
    ///
    /// 优化后的IR指令列表
    pub fn generate(
        &mut self,
        ast_node: &ASTNode<'t>,
    ) -> Result<Vec<(DebugInfo, IR)>, IRGeneratorError> {
        let irs = self.generate_without_redirect(ast_node)?;
        self.redirect_jump(irs)
    }
}
