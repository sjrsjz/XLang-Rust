use super::super::ir::{Functions, IROperation};
use crate::{
    parser::ast::{ASTNode, ASTNodeModifier, ASTNodeOperation, ASTNodeType},
    vm::ir::{DebugInfo, IR},
};
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
            namespace: namespace,
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

    fn generate_debug_info(ast_node: &ASTNode) -> IR {
        IR::DebugInfo(DebugInfo {
            code_position: match ast_node.token {
                Some(token) => token.position,
                None => 0,
            },
        })
    }

    pub fn generate_without_redirect(
        &mut self,
        ast_node: &ASTNode,
    ) -> Result<Vec<IR>, IRGeneratorError> {
        let debug_info = IRGenerator::generate_debug_info(&ast_node);
        match &ast_node.node_type {
            ASTNodeType::Body => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.push(IR::NewFrame);
                self.scope_stack.push(Scope::Frame);
                let children_len = ast_node.children.len();
                for i in 0..children_len {
                    let child = &ast_node.children[i];
                    let child_instructions = self.generate_without_redirect(child)?;
                    instructions.extend(child_instructions);
                }
                self.scope_stack.pop();
                instructions.push(IR::PopFrame);
                Ok(instructions)
            }
            ASTNodeType::LambdaDef => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                let args = &ast_node.children[0];
                let args_instructions = self.generate_without_redirect(&args)?;
                let ( full_signature,signature) = self.new_function_signature();

                let mut generator = IRGenerator::new(
                    self.functions,
                    NameSpace::new(signature.clone(), Some(&self.namespace)),
                );

                let mut body_instructions =
                    generator.generate(&ast_node.children[1])?; // body, compute redirect jump directly
                body_instructions.push(IR::Return);

                self.functions.append(signature.clone(), body_instructions);

                instructions.extend(args_instructions);
                if args.node_type != ASTNodeType::Tuple {
                    instructions.push(IR::BuildTuple(args.children.len()));
                }
                instructions.push(IR::LoadLambda(full_signature, ast_node.token.unwrap().position));
                Ok(instructions)
            }
            ASTNodeType::Assign => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::Set);
                Ok(instructions)
            }
            ASTNodeType::Variable(var_name) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.push(IR::Get(var_name.clone()));
                Ok(instructions)
            }
            ASTNodeType::Let(var_name) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push(IR::Let(var_name.clone()));
                Ok(instructions)
            }
            ASTNodeType::LambdaCall => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                instructions.push(IR::CallLambda);
                Ok(instructions)
            }
            ASTNodeType::Operation(opeartion) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                match opeartion {
                    ASTNodeOperation::Add => {
                        if ast_node.children.len() == 1 {
                            instructions.push(IR::UnaryOp(IROperation::Add));
                        } else {
                            instructions.push(IR::BinaryOp(IROperation::Add));
                        }
                    }
                    ASTNodeOperation::Subtract => {
                        if ast_node.children.len() == 1 {
                            instructions.push(IR::UnaryOp(IROperation::Subtract));
                        } else {
                            instructions.push(IR::BinaryOp(IROperation::Subtract));
                        }
                    }
                    ASTNodeOperation::Multiply => {
                        instructions.push(IR::BinaryOp(IROperation::Multiply));
                    }
                    ASTNodeOperation::Divide => {
                        instructions.push(IR::BinaryOp(IROperation::Divide));
                    }
                    ASTNodeOperation::Modulus => {
                        instructions.push(IR::BinaryOp(IROperation::Modulus));
                    }
                    ASTNodeOperation::And => {
                        instructions.push(IR::BinaryOp(IROperation::And));
                    }
                    ASTNodeOperation::Or => {
                        instructions.push(IR::BinaryOp(IROperation::Or));
                    }
                    ASTNodeOperation::Not => {
                        instructions.push(IR::UnaryOp(IROperation::Not));
                    }
                    ASTNodeOperation::Equal => {
                        instructions.push(IR::BinaryOp(IROperation::Equal));
                    }
                    ASTNodeOperation::NotEqual => {
                        instructions.push(IR::BinaryOp(IROperation::NotEqual));
                    }
                    ASTNodeOperation::Greater => {
                        instructions.push(IR::BinaryOp(IROperation::Greater));
                    }
                    ASTNodeOperation::Less => {
                        instructions.push(IR::BinaryOp(IROperation::Less));
                    }
                    ASTNodeOperation::GreaterEqual => {
                        instructions.push(IR::BinaryOp(IROperation::GreaterEqual));
                    }
                    ASTNodeOperation::LessEqual => {
                        instructions.push(IR::BinaryOp(IROperation::LessEqual));
                    }
                    ASTNodeOperation::BitwiseAnd => {
                        instructions.push(IR::BinaryOp(IROperation::BitwiseAnd));
                    }
                    ASTNodeOperation::BitwiseOr => {
                        instructions.push(IR::BinaryOp(IROperation::BitwiseOr));
                    }
                    ASTNodeOperation::BitwiseXor => {
                        instructions.push(IR::BinaryOp(IROperation::BitwiseXor));
                    }
                    ASTNodeOperation::ShiftLeft => {
                        instructions.push(IR::BinaryOp(IROperation::ShiftLeft));
                    }
                    ASTNodeOperation::ShiftRight => {
                        instructions.push(IR::BinaryOp(IROperation::ShiftRight));
                    }
                    ASTNodeOperation::Power => {
                        instructions.push(IR::BinaryOp(IROperation::Power));
                    }
                }
                Ok(instructions)
            }
            ASTNodeType::IndexOf => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::IndexOf);
                Ok(instructions)
            }
            ASTNodeType::GetAttr => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::GetAttr);
                Ok(instructions)
            }
            ASTNodeType::Return => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push(IR::Return);
                Ok(instructions)
            }
            ASTNodeType::Number(number_str) => {
                // check if number_str is float or int
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                if let Ok(number) = number_str.parse::<i32>() {
                    instructions.push(IR::LoadInt(number));
                } else if let Ok(number) = number_str.parse::<f64>() {
                    instructions.push(IR::LoadFloat(number));
                } else {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
                Ok(instructions)
            }
            ASTNodeType::Tuple => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                let mut tuple_size = 0;
                for child in &ast_node.children {
                    if child.node_type == ASTNodeType::None {
                        continue;
                    }
                    instructions.extend(self.generate_without_redirect(child)?);
                    tuple_size += 1;
                }
                instructions.push(IR::BuildTuple(tuple_size));
                Ok(instructions)
            }
            ASTNodeType::KeyValue => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::BuildKeyValue);
                Ok(instructions)
            }
            ASTNodeType::NamedTo => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::BuildNamed);
                Ok(instructions)
            }
            ASTNodeType::String(str) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.push(IR::LoadString(str.clone()));
                Ok(instructions)
            }
            ASTNodeType::Expressions => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                for child in &ast_node.children {
                    instructions.extend(self.generate_without_redirect(child)?);
                }
                Ok(instructions)
            }
            ASTNodeType::Null => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.push(IR::LoadNull);
                Ok(instructions)
            }
            ASTNodeType::None => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                instructions.push(IR::LoadNull);
                Ok(instructions)
            }
            ASTNodeType::Boolean(bool_str) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                if bool_str == "true" {
                    instructions.push(IR::LoadBool(true));
                } else if bool_str == "false" {
                    instructions.push(IR::LoadBool(false));
                } else {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
                Ok(instructions)
            }
            ASTNodeType::If => match ast_node.children.len() {
                0..=1 => {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
                2 => {
                    let mut instructions = Vec::new();
                    instructions.push(debug_info);
                    instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                    let body_instructions =
                        self.generate_without_redirect(&ast_node.children[1])?;
                    let (if_label, _) = self.new_label();
                    let (else_label, _) = self.new_label();
                    instructions.push(IR::RedirectJumpIfFalse(if_label.clone()));
                    instructions.extend(body_instructions);
                    instructions.push(IR::RedirectJump(else_label.clone()));
                    instructions.push(IR::RedirectLabel(if_label.clone()));
                    instructions.push(IR::LoadNull);
                    instructions.push(IR::RedirectLabel(else_label.clone()));
                    Ok(instructions)
                }
                3 => {
                    let mut instructions = Vec::new();
                    instructions.push(debug_info);
                    instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                    let body_instructions =
                        self.generate_without_redirect(&ast_node.children[1])?;
                    let else_body_instructions =
                        self.generate_without_redirect(&ast_node.children[2])?;
                    let (if_label, _) = self.new_label();
                    let (else_label, _) = self.new_label();
                    instructions.push(IR::RedirectJumpIfFalse(if_label.clone()));
                    instructions.extend(body_instructions);
                    instructions.push(IR::RedirectJump(else_label.clone()));
                    instructions.push(IR::RedirectLabel(if_label.clone()));
                    instructions.extend(else_body_instructions);
                    instructions.push(IR::RedirectLabel(else_label.clone()));
                    Ok(instructions)
                }
                _ => {
                    return Err(IRGeneratorError::InvalidASTNodeType(
                        ast_node.node_type.clone(),
                    ));
                }
            },
            ASTNodeType::Break => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
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
                    instructions.push(IR::PopFrame);
                }
                instructions.push(IR::RedirectJump(loop_label.unwrap()));

                Ok(instructions)
            }
            ASTNodeType::Continue => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
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

                instructions.push(IR::RedirectJump(loop_label.unwrap()));
                Ok(instructions)
            }
            ASTNodeType::While => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                let (head_label, _) = self.new_label();
                let (end_label, _) = self.new_label();
                self.scope_stack
                    .push(Scope::Loop(head_label.clone(), end_label.clone()));
                instructions.push(IR::RedirectLabel(head_label.clone()));
                instructions.extend(self.generate_without_redirect(&ast_node.children[0])?);
                instructions.push(IR::RedirectJumpIfFalse(head_label.clone()));
                instructions.extend(self.generate_without_redirect(&ast_node.children[1])?);
                instructions.push(IR::RedirectJump(head_label.clone()));
                instructions.push(IR::RedirectLabel(end_label.clone()));
                self.scope_stack.pop();
                Ok(instructions)
            }
            ASTNodeType::Modifier(modifier) => {
                let mut instructions = Vec::new();
                instructions.push(debug_info);
                match modifier {
                    ASTNodeModifier::SelfOf => {
                        instructions.push(IR::SelfOf);
                    }
                    ASTNodeModifier::KeyOf => {
                        instructions.push(IR::KeyOf);
                    }
                    ASTNodeModifier::ValueOf => {
                        instructions.push(IR::ValueOf);
                    }
                    ASTNodeModifier::Ref => {
                        instructions.push(IR::RefValue);
                    }
                    ASTNodeModifier::Deref => {
                        instructions.push(IR::DerefValue);
                    }
                    ASTNodeModifier::Assert => {
                        instructions.push(IR::Assert);
                    }
                    ASTNodeModifier::Copy => {
                        instructions.push(IR::CopyValue);
                    }
                    ASTNodeModifier::Import => {
                        instructions.push(IR::Import(ast_node.children[0].token.unwrap().position));
                    }
                    ASTNodeModifier::TypeOf => {
                        instructions.push(IR::TypeOf);
                    }
                }
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
    fn redirect_jump(&self, irs: Vec<IR>) -> Result<Vec<IR>, IRGeneratorError> {
        let mut reduced_irs = Vec::new();
        let mut label_map = std::collections::HashMap::new();
        
        // 首先收集所有标签的位置
        for (i, ir) in irs.iter().enumerate() {
            if let IR::RedirectLabel(label) = ir {
                label_map.insert(label.clone(), reduced_irs.len());
            } else {
                reduced_irs.push(ir.clone());
            }
        }
        
        // 转换所有跳转指令
        for i in 0..reduced_irs.len() {
            match &reduced_irs[i] {
                IR::RedirectJump(label) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize;
                        reduced_irs[i] = IR::JumpOffset(offset);
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                },
                IR::RedirectJumpIfFalse(label) => {
                    if let Some(&target_pos) = label_map.get(label) {
                        let offset = target_pos as isize - i as isize;
                        reduced_irs[i] = IR::JumpIfFalseOffset(offset);
                    } else {
                        return Err(IRGeneratorError::InvalidLabel);
                    }
                },
                _ => {}
            }
        }
        
        Ok(reduced_irs)
    }
    
    /// 移除相邻的debug_info指令，只保留最新的一个
    ///
    /// # Arguments
    ///
    /// * `irs` - 原始IR指令列表
    ///
    /// # Returns
    ///
    /// 处理后的IR指令列表
    fn retain_latest_debug_info(&self, irs: Vec<IR>) -> Vec<IR> {
        if irs.is_empty() {
            return Vec::new();
        }
        
        let mut result = Vec::new();
        let mut last_was_debug = false;
        
        for ir in irs {
            match ir {
                IR::DebugInfo(_) => {
                    if last_was_debug {
                        // 如果前一条也是DEBUG_INFO，则替换它
                        *result.last_mut().unwrap() = ir;
                    } else {
                        // 否则添加此DEBUG_INFO
                        result.push(ir);
                        last_was_debug = true;
                    }
                },
                _ => {
                    // 非DEBUG_INFO指令直接添加
                    result.push(ir);
                    last_was_debug = false;
                }
            }
        }
        
        result
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
    pub fn generate(&mut self, ast_node: &ASTNode<'t>) -> Result<Vec<IR>, IRGeneratorError> {
        let irs = self.generate_without_redirect(ast_node)?;
        let irs = self.retain_latest_debug_info(irs);
        self.redirect_jump(irs)
    }
}
