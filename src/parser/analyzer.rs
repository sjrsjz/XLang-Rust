use super::ast::ASTNode;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Variable {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub struct VariableFrame {
    pub variables: Vec<Variable>,
}

#[derive(Debug)]
pub struct VariableContext {
    pub frames: Vec<VariableFrame>,
}

#[derive(Debug)]
pub enum AnalyzeError<'t> {
    UndefinedVariable(&'t ASTNode<'t>),
}

impl AnalyzeError<'_> {
    pub fn format(&self, source_code: String) -> String {
        use super::ast::ASTNodeType::Variable;
        use colored::*;
        use unicode_segmentation::UnicodeSegmentation;

        // 分割源代码为行
        let lines: Vec<&str> = source_code.lines().collect();

        // Helper function to find line and column from position
        let find_position = |byte_pos: usize| -> (usize, usize) {
            let mut current_byte = 0;
            for (line_num, line) in lines.iter().enumerate() {
                // 计算行长度（包括换行符）
                // Windows通常使用CRLF (\r\n)，而Unix使用LF (\n)
                // 我们需要检测使用的是哪种换行符
                let eol_len = if source_code.contains("\r\n") { 2 } else { 1 };
                let line_bytes = line.len() + eol_len; // 加上实际的换行符长度

                if current_byte + line_bytes > byte_pos {
                    // 计算行内的字节偏移
                    let line_offset = byte_pos - current_byte;

                    // 边界检查
                    if line_offset > line.len() {
                        return (line_num, line.graphemes(true).count()); // 位置在行尾
                    }

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
            (lines.len().saturating_sub(1), 0) // Default to last line
        };

        match self {
            AnalyzeError::UndefinedVariable(node) => {
                let (line_num, col) = find_position(match node.token {
                    Some(node) => node.position,
                    None => 0,
                });
                let line = if line_num < lines.len() {
                    lines[line_num]
                } else {
                    ""
                };

                let var_name = if let Variable(name) = &node.node_type {
                    name
                } else {
                    "unknown"
                };

                let mut warning_msg = format!(
                    "{}: {}\n\n",
                    "Analysis Error".bright_red().bold(),
                    format!("Undefined variable '{}'", var_name).red()
                );
                warning_msg.push_str(&format!(
                    "{} {}:{}\n",
                    "Position".bright_blue(),
                    (line_num + 1).to_string().bright_cyan(),
                    (col + 1).to_string().bright_cyan()
                ));
                warning_msg.push_str(&format!("{}\n", line.white()));

                // 计算节点在源代码中的长度
                let node_length = var_name.len();
                warning_msg.push_str(&format!(
                    "{}{}\n",
                    " ".repeat(col),
                    "^".repeat(node_length).bright_red().bold()
                ));

                // 添加建议提示
                warning_msg.push_str(&format!(
                    "\n{} {}\n",
                    "Hint:".bright_green().bold(),
                    format!("Variable '{}' is used but not defined in the current scope, if the variable is dynamic, use `dynamic` annotation.", var_name).bright_white()
                        .italic()
                ));

                warning_msg
            }
        }
    }
}

#[derive(Debug)]
pub struct AnalyzeResult<'t> {
    pub errors: Vec<AnalyzeError<'t>>,
}

impl VariableContext {
    pub fn new() -> Self {
        VariableContext {
            frames: vec![VariableFrame {
                variables: Vec::new(),
            }],
        }
    }

    pub fn define_variable(&mut self, var: &Variable) -> Result<(), String> {
        if let Some(frame) = self.frames.last_mut() {
            frame.variables.push(var.clone());
        } else {
            return Err("No frame available to define variable".to_string());
        }
        Ok(())
    }

    pub fn get_variable(&self, name: &str) -> Option<&Variable> {
        for frame in self.frames.iter().rev() {
            if let Some(var) = frame.variables.iter().find(|v| v.name == name) {
                return Some(var);
            }
        }
        None
    }

    pub fn push_frame(&mut self) {
        self.frames.push(VariableFrame {
            variables: Vec::new(),
        });
    }

    pub fn pop_frame(&mut self) -> Result<(), String> {
        if self.frames.len() > 1 {
            self.frames.pop();
            Ok(())
        } else {
            Err("Cannot pop the global frame".to_string())
        }
    }
}

pub fn analyze_ast<'t>(ast: &'t ASTNode) -> AnalyzeResult<'t> {
    let mut context = VariableContext::new();
    let mut warnings = Vec::new();

    analyze_node(ast, &mut context, &mut warnings, false);

    AnalyzeResult { errors: warnings }
}

fn analyze_node<'t>(
    node: &'t ASTNode,
    context: &mut VariableContext,
    warnings: &mut Vec<AnalyzeError<'t>>,
    dynamic: bool,
) {
    use super::ast::ASTNodeType;

    match &node.node_type {
        ASTNodeType::Let(var_name) => {
            // 处理变量定义
            if let Some(value_node) = node.children.first() {
                // 对于递归函数，先定义变量再分析其值
                let is_lambda = matches!(value_node.node_type, ASTNodeType::LambdaDef(_));

                // 如果是lambda函数定义，先注册变量名，以支持递归
                if is_lambda {
                    let var = Variable {
                        name: var_name.clone(),
                        value: "".to_string(),
                    };
                    let _ = context.define_variable(&var);
                }

                // 分析赋值表达式
                analyze_node(value_node, context, warnings, dynamic);

                // 如果不是lambda函数，按原方式处理
                if !is_lambda {
                    let var = Variable {
                        name: var_name.clone(),
                        value: "".to_string(), // 实际值在运行时才能确定
                    };
                    let _ = context.define_variable(&var);
                }
            }
        }
        ASTNodeType::Annotation(annotation) => {
            match annotation.as_str() {
                "dynamic" => {
                    for child in &node.children {
                        analyze_node(child, context, warnings, true);
                    }
                }
                "static" => {
                    for child in &node.children {
                        analyze_node(child, context, warnings, false);
                    }
                }
                _ => {
                    // 处理其他注解类型
                    for child in &node.children {
                        analyze_node(child, context, warnings, dynamic);
                    }
                }
            }
        }

        ASTNodeType::Variable(var_name) => {
            // 检查变量是否定义
            if context.get_variable(var_name).is_none()
                && ![
                    "this",
                    "self",
                    "len",
                    "int",
                    "float",
                    "string",
                    "bool",
                    "bytes",
                    "print",
                    "input",
                    "load_clambda",
                ]
                .contains(&var_name.as_str())
            {
                if !dynamic {
                    warnings.push(AnalyzeError::UndefinedVariable(node));
                }
            }
        }
        ASTNodeType::Body | ASTNodeType::Boundary => {
            // 创建新的作用域
            context.push_frame();

            // 分析所有子节点
            for child in &node.children {
                analyze_node(child, context, warnings, dynamic);
            }

            // 离开作用域
            let _ = context.pop_frame();
        }
        ASTNodeType::LambdaDef(_) => {
            // 处理Lambda定义，创建新的作用域
            context.push_frame();

            // 处理参数列表（第一个子节点）
            if let Some(params) = node.children.first() {
                analyze_tuple_params(params, context, warnings, dynamic);
            }

            // 分析函数体（第二个子节点）
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings, dynamic);
            }

            // 离开函数作用域
            let _ = context.pop_frame();
        }
        ASTNodeType::LambdaCall => {
            // 分析函数调用
            // 先分析被调用的函数
            if let Some(func_node) = node.children.first() {
                analyze_node(func_node, context, warnings, dynamic);
            }

            // 分析参数
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings, dynamic);
            }
        }
        ASTNodeType::Expressions => {
            // 处理多个表达式
            for child in &node.children {
                analyze_node(child, context, warnings, dynamic);
            }
        }
        ASTNodeType::Assign => {
            // 赋值操作
            if node.children.len() >= 2 {
                // 先分析右侧表达式
                analyze_node(&node.children[1], context, warnings, dynamic);

                analyze_node(&node.children[0], context, warnings, dynamic);
            }
        }
        ASTNodeType::If => {
            // 分析条件
            if let Some(condition) = node.children.first() {
                analyze_node(condition, context, warnings, dynamic);
            }

            // 分析 then 块
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings, dynamic);
            }

            // 分析 else 块（如果存在）
            if node.children.len() > 2 {
                analyze_node(&node.children[2], context, warnings, dynamic);
            }
        }
        ASTNodeType::While => {
            // 分析循环条件
            if let Some(condition) = node.children.first() {
                analyze_node(condition, context, warnings, dynamic);
            }

            // 分析循环体
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings, dynamic);
            }
        }
        ASTNodeType::Return | ASTNodeType::Yield | ASTNodeType::Raise => {
            // 分析返回值
            if let Some(value) = node.children.first() {
                analyze_node(value, context, warnings, dynamic);
            }
        }
        ASTNodeType::Operation(_) => {
            // 分析操作符两侧的表达式
            for child in &node.children {
                analyze_node(child, context, warnings, dynamic);
            }
        }
        ASTNodeType::Tuple => {
            // 分析元组的每个元素
            for child in &node.children {
                analyze_node(child, context, warnings, dynamic);
            }
        }
        ASTNodeType::GetAttr => {
            // 分析对象和属性
            if node.children.len() >= 2 {
                analyze_node(&node.children[0], context, warnings, dynamic); // 对象
                analyze_node(&node.children[1], context, warnings, dynamic);
                // 属性
            }
        }
        ASTNodeType::IndexOf => {
            // 分析索引操作
            if node.children.len() >= 2 {
                analyze_node(&node.children[0], context, warnings, dynamic); // 被索引的对象
                analyze_node(&node.children[1], context, warnings, dynamic);
                // 索引值
            }
        }
        ASTNodeType::Modifier(_) => {
            // 分析修饰器的目标
            if let Some(target) = node.children.first() {
                analyze_node(target, context, warnings, dynamic);
            }
        }
        // 其他简单数据类型不需要特殊处理
        ASTNodeType::String(_)
        | ASTNodeType::Boolean(_)
        | ASTNodeType::Number(_)
        | ASTNodeType::Base64(_)
        | ASTNodeType::Null => {
            // 不需要特殊处理
        }
        // 其他节点类型的通用处理（递归处理所有子节点）
        _ => {
            for child in &node.children {
                analyze_node(child, context, warnings, dynamic);
            }
        }
    }
}

// 分析元组参数，用于函数定义
fn analyze_tuple_params<'t>(
    params: &'t ASTNode,
    context: &mut VariableContext,
    warnings: &mut Vec<AnalyzeError<'t>>,
    dynamic: bool,
) {
    use super::ast::ASTNodeType;

    if let ASTNodeType::Tuple = params.node_type {
        for param in &params.children {
            match &param.node_type {
                ASTNodeType::Variable(var_name) => {
                    // 在函数作用域中注册参数
                    let var = Variable {
                        name: var_name.clone(),
                        value: "".to_string(), // 参数值在运行时确定
                    };
                    let _ = context.define_variable(&var);
                }
                ASTNodeType::NamedTo => {
                    // 处理默认参数 param => default_value
                    if param.children.len() >= 2 {
                        // 分析默认值表达式
                        analyze_node(&param.children[1], context, warnings, dynamic);

                        // 注册参数名
                        if let ASTNodeType::Variable(var_name) = &param.children[0].node_type {
                            let var = Variable {
                                name: var_name.clone(),
                                value: "".to_string(),
                            };
                            let _ = context.define_variable(&var);
                        } else if let ASTNodeType::String(var_name) = &param.children[0].node_type {
                            let var = Variable {
                                name: var_name.clone(),
                                value: "".to_string(),
                            };
                            let _ = context.define_variable(&var);
                        } else {
                            // 分析更复杂的参数结构
                            analyze_node(&param.children[0], context, warnings, dynamic);
                        }
                    }
                }
                // 处理其他类型的参数定义
                _ => analyze_node(param, context, warnings, dynamic),
            }
        }
    } else if let ASTNodeType::AssumeTuple = params.node_type {
        // 处理可变参数
        if let Some(param) = params.children.first() {
            analyze_node(param, context, warnings, dynamic);
        }
    } else {
        // 单参数函数
        if let ASTNodeType::Variable(var_name) = &params.node_type {
            let var = Variable {
                name: var_name.clone(),
                value: "".to_string(),
            };
            let _ = context.define_variable(&var);
        } else {
            // 其他复杂参数类型
            analyze_node(params, context, warnings, dynamic);
        }
    }
}
