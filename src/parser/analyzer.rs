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
pub enum AnalyzeWarn<'t> {
    UndefinedVariable(&'t ASTNode<'t>),
}

#[derive(Debug)]
pub struct AnalyzeResult<'t> {
    pub warnings: Vec<AnalyzeWarn<'t>>,
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

    analyze_node(ast, &mut context, &mut warnings);

    AnalyzeResult { warnings }
}

fn analyze_node<'t>(
    node: &'t ASTNode,
    context: &mut VariableContext,
    warnings: &mut Vec<AnalyzeWarn<'t>>,
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
                analyze_node(value_node, context, warnings);

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
                warnings.push(AnalyzeWarn::UndefinedVariable(node));
            }
        }
        ASTNodeType::Body | ASTNodeType::Boundary => {
            // 创建新的作用域
            context.push_frame();

            // 分析所有子节点
            for child in &node.children {
                analyze_node(child, context, warnings);
            }

            // 离开作用域
            let _ = context.pop_frame();
        }
        ASTNodeType::LambdaDef(_) => {
            // 处理Lambda定义，创建新的作用域
            context.push_frame();

            // 处理参数列表（第一个子节点）
            if let Some(params) = node.children.first() {
                analyze_tuple_params(params, context, warnings);
            }

            // 分析函数体（第二个子节点）
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings);
            }

            // 离开函数作用域
            let _ = context.pop_frame();
        }
        ASTNodeType::LambdaCall => {
            // 分析函数调用
            // 先分析被调用的函数
            if let Some(func_node) = node.children.first() {
                analyze_node(func_node, context, warnings);
            }

            // 分析参数
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings);
            }
        }
        ASTNodeType::Expressions => {
            // 处理多个表达式
            for child in &node.children {
                analyze_node(child, context, warnings);
            }
        }
        ASTNodeType::Assign => {
            // 赋值操作
            if node.children.len() >= 2 {

                // 先分析右侧表达式
                analyze_node(&node.children[1], context, warnings);

                analyze_node(&node.children[0], context, warnings);
            }
        }
        ASTNodeType::If => {
            // 分析条件
            if let Some(condition) = node.children.first() {
                analyze_node(condition, context, warnings);
            }

            // 分析 then 块
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings);
            }

            // 分析 else 块（如果存在）
            if node.children.len() > 2 {
                analyze_node(&node.children[2], context, warnings);
            }
        }
        ASTNodeType::While => {
            // 分析循环条件
            if let Some(condition) = node.children.first() {
                analyze_node(condition, context, warnings);
            }

            // 分析循环体
            if node.children.len() > 1 {
                analyze_node(&node.children[1], context, warnings);
            }
        }
        ASTNodeType::Return | ASTNodeType::Yield | ASTNodeType::Raise => {
            // 分析返回值
            if let Some(value) = node.children.first() {
                analyze_node(value, context, warnings);
            }
        }
        ASTNodeType::Operation(_) => {
            // 分析操作符两侧的表达式
            for child in &node.children {
                analyze_node(child, context, warnings);
            }
        }
        ASTNodeType::Tuple => {
            // 分析元组的每个元素
            for child in &node.children {
                analyze_node(child, context, warnings);
            }
        }
        ASTNodeType::GetAttr => {
            // 分析对象和属性
            if node.children.len() >= 2 {
                analyze_node(&node.children[0], context, warnings); // 对象
                analyze_node(&node.children[1], context, warnings); // 属性
            }
        }
        ASTNodeType::IndexOf => {
            // 分析索引操作
            if node.children.len() >= 2 {
                analyze_node(&node.children[0], context, warnings); // 被索引的对象
                analyze_node(&node.children[1], context, warnings); // 索引值
            }
        }
        ASTNodeType::Modifier(_) => {
            // 分析修饰器的目标
            if let Some(target) = node.children.first() {
                analyze_node(target, context, warnings);
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
                analyze_node(child, context, warnings);
            }
        }
    }
}

// 分析元组参数，用于函数定义
fn analyze_tuple_params<'t>(
    params: &'t ASTNode,
    context: &mut VariableContext,
    warnings: &mut Vec<AnalyzeWarn<'t>>,
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
                        analyze_node(&param.children[1], context, warnings);

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
                            analyze_node(&param.children[0], context, warnings);
                        }
                    }
                }
                // 处理其他类型的参数定义
                _ => analyze_node(param, context, warnings),
            }
        }
    } else if let ASTNodeType::AssumeTuple = params.node_type {
        // 处理可变参数
        if let Some(param) = params.children.first() {
            analyze_node(param, context, warnings);
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
            analyze_node(params, context, warnings);
        }
    }
}
