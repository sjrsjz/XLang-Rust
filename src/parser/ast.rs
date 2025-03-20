use std::{fmt::Debug, vec};

use crate::{Token, TokenType};

#[derive(Debug)]
pub enum ParserError<'t> {
    UnexpectedToken(&'t Token<'t>),                     // Token
    UnmatchedParenthesis(&'t Token<'t>, &'t Token<'t>), // (opening, closing)
    InvalidSyntax(&'t Token<'t>),
    NotFullyMatched(&'t Token<'t>, &'t Token<'t>),
    InvalidVariableName(&'t Token<'t>),
    UnsupportedStructure(&'t Token<'t>),
}

impl ParserError<'_> {
    pub fn format(&self, tokens: &Vec<Token>) -> String {
        match self {
            ParserError::UnexpectedToken(token) => {
                format!(
                    "解析错误: 意外的令牌 '{}' 在位置 {}",
                    token.token, token.position
                )
            }
            ParserError::UnmatchedParenthesis(opening, closing) => {
                let mut code = String::new();
                let opening_idx = tokens
                    .iter()
                    .position(|t| t.position == opening.position)
                    .unwrap();
                let closing_idx = tokens
                    .iter()
                    .position(|t| t.position == closing.position)
                    .unwrap();
                for token in tokens[opening_idx..=closing_idx].iter() {
                    code.push_str(&token.token);
                }
                format!(
                    "解析错误: 括号不匹配 '{}' 在位置 {}",
                    code, opening.position
                )
            }
            ParserError::InvalidSyntax(token) => {
                format!("语法错误: 无效的语法在位置 {}", token.position)
            }
            ParserError::NotFullyMatched(start, end) => {
                let mut code = String::new();
                let start_idx = tokens
                    .iter()
                    .position(|t| t.position == start.position)
                    .unwrap();
                let end_idx = tokens
                    .iter()
                    .position(|t| t.position == end.position)
                    .unwrap();
                for token in tokens[start_idx..=end_idx].iter() {
                    code.push_str(&(token.token.to_string() + " "));
                }
                format!("解析错误: 未完全匹配 '{}' 在位置 {}", code, start.position)
            }
            ParserError::InvalidVariableName(token) => {
                format!(
                    "解析错误: 无效的变量名 '{}' 在位置 {}",
                    token.token, token.position
                )
            }
            ParserError::UnsupportedStructure(token) => {
                format!("解析错误: 不支持的结构在位置 {}", token.position)
            }
        }
    }
}

pub type TokenStream<'t> = Vec<Token<'t>>;
pub type GatheredTokens<'t> = &'t [Token<'t>];

pub mod ast_token_stream {
    pub fn from_stream<'t>(stream: &'t super::TokenStream<'t>) -> super::GatheredTokens<'t> {
        stream.as_slice()
    }
}

fn get_next_tokens<'a>(
    tokens: GatheredTokens<'a>,
    current: usize,
) -> Result<GatheredTokens<'a>, ParserError<'a>> {
    let mut stack = Vec::<(&str, usize)>::new();
    let mut next_tokens_end = 0usize;
    let mut index = current;
    if index >= (*tokens).len() {
        return Ok(&[]);
    }
    loop {
        if vec!["{", "[", "("].contains(&tokens[index].token)
            && tokens[index].token_type == TokenType::SYMBOL
        {
            stack.push((tokens[index].token, index));
            next_tokens_end += 1;
        } else if vec!["}", "]", ")"].contains(&tokens[index].token)
            && tokens[index].token_type == TokenType::SYMBOL
        {
            if stack.len() == 0 {
                break;
            }
            let (last, last_position) = stack.pop().unwrap();
            if (last == "{" && tokens[index].token != "}")
                || (last == "[" && tokens[index].token != "]")
                || (last == "(" && tokens[index].token != ")")
            {
                return Err(ParserError::UnmatchedParenthesis(
                    &tokens[last_position],
                    &tokens[index],
                ));
            }

            next_tokens_end += 1;
        } else {
            next_tokens_end += 1;
        }
        index += 1;
        if index >= (tokens).len() || stack.len() == 0 {
            break;
        }
    }
    if stack.len() > 0 {
        let (_, last_position) = stack.pop().unwrap();
        return Err(ParserError::UnmatchedParenthesis(
            &tokens[last_position],
            &tokens[index - 1],
        ));
    }
    return Ok(&tokens[current..current + next_tokens_end]);
}

fn gather<'t>(tokens: GatheredTokens<'t>) -> Result<Vec<GatheredTokens<'t>>, ParserError<'t>> {
    let mut current = 0;
    let mut result = Vec::<GatheredTokens>::new();
    while current < tokens.len() {
        let next_tokens = get_next_tokens(tokens, current)?;
        if next_tokens.len() == 0 {
            return Err(ParserError::UnsupportedStructure(&tokens[current]));
        }
        current += next_tokens.len();
        result.push(next_tokens);
    }
    Ok(result)
}

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNodeType {
    None,                        // No expression
    Null,                        // Null
    String(String),              // String
    Boolean(String),             // Boolean
    Number(String),              // Number (Integer, Float)
    Variable(String),            // Variable
    Let(String),                 // x := expression
    Body,                        // {...}
    Assign,                      // x = expression
    LambdaDef,                   // tuple -> body
    Expressions,                 // expression1; expression2; ...
    LambdaCall,                  // x (tuple)
    Operation(ASTNodeOperation), // x + y, x - y, x * y, x / y ...
    Tuple,                       // x, y, z, ...
    KeyValue,                    // x: y
    IndexOf,                     // x[y]
    GetAttr,                     // x.y
    Return,                      // return expression
    If,    // if expression truecondition || if expression truecondition else falsecondition
    While, // while expression body
    Modifier(ASTNodeModifier), // modifier expression
    NamedTo, // x => y (x is name of y)
    Break, // break
    Continue, // continue
    Range, // x..y
    In,
    Yield,
    AsyncLambdaCall,
    Alias(String), // Type::Value
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ASTNodeOperation {
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Modulus,      // %
    Power,        // ^
    BitwiseAnd,   // &
    BitwiseOr,    // |
    BitwiseXor,   // ^
    ShiftLeft,    // <<
    ShiftRight,   // >>
    And,          // and
    Or,           // or
    Not,          // not
    Equal,        // ==
    NotEqual,     // !=
    Greater,      // >
    Less,         // <
    GreaterEqual, // >=
    LessEqual,    // <=
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ASTNodeModifier {
    DeepCopy, // DeepCopy
    Copy,     // Copy
    Ref,      // Ref
    Deref,    // Deref
    KeyOf,    // KeyOf
    ValueOf,  // ValueOf
    SelfOf,   // SelfOf
    Assert,   // Assert
    Import,   // Import
    TypeOf,   // TypeOf
    Wrap,     // Wrap
    Await,
    Wipe,
    AliasOf,
    BindSelf,
}

#[derive(Debug)]
pub struct ASTNode<'t> {
    pub node_type: ASTNodeType,       // Type of the node
    pub token: Option<&'t Token<'t>>, // Token associated with the node
    pub children: Vec<ASTNode<'t>>,   // Children of the node
}

impl ASTNode<'_> {
    pub fn new<'t>(
        node_type: ASTNodeType,
        token: Option<&'t Token>,
        children: Option<Vec<ASTNode<'t>>>,
    ) -> ASTNode<'t> {
        ASTNode {
            node_type,
            token: match token {
                Some(token) => Some(token),
                None => None,
            },
            children: match children {
                Some(children) => children,
                None => Vec::new(),
            },
        }
    }

    pub fn formatted_print(&self, indent: usize) {
        let indent_str = " ".repeat(indent);
        let output = match &self.node_type {
            node_type @ (ASTNodeType::Variable(v)
            | ASTNodeType::Number(v)
            | ASTNodeType::String(v)
            | ASTNodeType::Boolean(v)) => {
                format!("{}{:?}: {:?}", indent_str, node_type, v)
            }
            node_type @ _ => format!("{}{:?}", indent_str, node_type),
        };

        println!("{}", output);

        if self.children.len() > 0 {
            for child in &self.children {
                child.formatted_print(indent + 2);
            }
        }
    }
}

type MatcherFn<'a> = Box<
    dyn Fn(
        &Vec<GatheredTokens<'a>>,
        usize,
    ) -> Result<(Option<ASTNode<'a>>, usize), ParserError<'a>>,
>;

struct NodeMatcher<'a> {
    matchers: Vec<MatcherFn<'a>>,
}

impl<'a> NodeMatcher<'a> {
    fn new() -> NodeMatcher<'a> {
        NodeMatcher {
            matchers: Vec::new(),
        }
    }

    fn add_matcher(&mut self, matcher: MatcherFn<'a>) {
        self.matchers.push(matcher);
    }

    fn match_node<'b>(
        &self,
        tokens: &'b Vec<GatheredTokens<'a>>,
        current: usize,
    ) -> Result<(Option<ASTNode<'a>>, usize), ParserError<'a>> {
        if tokens.len() == 0 {
            return Ok((Some(ASTNode::new(ASTNodeType::None, None, None)), 0));
        }
        for matcher in &self.matchers {
            if current >= tokens.len() {
                return Ok((None, 0));
            }
            let (node, offset) = matcher(tokens, current)?;
            if node.is_some() {
                return Ok((node, offset));
            }
        }
        Ok((None, 0))
    }
}

fn is_symbol(token: &GatheredTokens, symbol: &str) -> bool {
    if token.len() != 1 {
        return false;
    }
    let token = &token[0];
    token.token_type == TokenType::SYMBOL && token.token == symbol
}

fn is_identifier(token: &GatheredTokens, identifier: &str) -> bool {
    if token.len() != 1 {
        return false;
    }
    let token = &token[0];
    token.token_type == TokenType::IDENTIFIER && token.token == identifier
}

fn unwrap_brace<'t>(token: &GatheredTokens<'t>) -> Result<GatheredTokens<'t>, ParserError<'t>> {
    if token.len() < 2 {
        return Err(ParserError::UnexpectedToken(&token[0]));
    }
    if token[0].token_type == TokenType::SYMBOL
        && token[0].token == "{"
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == "}"
    {
        return Ok(&token[1..token.len() - 1]);
    }
    if token[0].token_type == TokenType::SYMBOL
        && token[0].token == "["
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == "]"
    {
        return Ok(&token[1..token.len() - 1]);
    }
    if token[0].token_type == TokenType::SYMBOL
        && token[0].token == "("
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == ")"
    {
        return Ok(&token[1..token.len() - 1]);
    }
    return Err(ParserError::UnexpectedToken(&token[0]));
}

fn is_bracket(token: &GatheredTokens) -> bool {
    if token.len() < 2 {
        return false;
    }
    return token[0].token_type == TokenType::SYMBOL
        && token[0].token == "("
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == ")";
}

fn is_brace(token: &GatheredTokens) -> bool {
    if token.len() < 2 {
        return false;
    }
    return token[0].token_type == TokenType::SYMBOL
        && token[0].token == "{"
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == "}";
}

fn is_square_bracket(token: &GatheredTokens) -> bool {
    if token.len() < 2 {
        return false;
    }
    return token[0].token_type == TokenType::SYMBOL
        && token[0].token == "["
        && token[token.len() - 1].token_type == TokenType::SYMBOL
        && token[token.len() - 1].token == "]";
}

pub fn build_ast<'a>(tokens: GatheredTokens<'a>) -> Result<ASTNode<'a>, ParserError<'a>> {
    let gathered = gather(&tokens)?;
    let (matched, offset) = match_all(&gathered, 0)?;
    if matched.is_none() {
        return Err(ParserError::InvalidSyntax(&tokens[0]));
    }
    let matched = matched.unwrap();
    if offset != gathered.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[0],
            &tokens[tokens.len() - 1],
        ));
    }
    return Ok(matched);
}

fn match_all<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut node_matcher = NodeMatcher::new();
    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_expressions(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_return_and_yield(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens: &Vec<&[Token<'_>]>,
         current|
         -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_tuple(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_let(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_assign(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_named_to(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_key_value(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_while(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_control_flow(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_if(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_or(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_and(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_not(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_operation_compare(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_operation_add_sub(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_operation_mul_div_mod(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_lambda_def(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_range(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_in(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_modifier(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_named_to_null(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_alias(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_member_access_and_call(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
            match_variable(tokens, current)
        },
    ));

    return node_matcher.match_node(tokens, current);
}

fn match_expressions<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset = 0usize;
    let mut left_tokens = Vec::<GatheredTokens>::new();
    let mut last_offset = 0usize;
    let mut separated = Vec::<ASTNode>::new();
    while current + offset < tokens.len() {
        if is_symbol(&tokens[current + offset], ";") {
            let (node, node_offset) = match_all(&left_tokens, 0)?;
            if node.is_none() {
                return Ok((None, 0));
            }
            if node_offset != left_tokens.len() {
                return Err(ParserError::NotFullyMatched(
                    &tokens[current][0],
                    &tokens[current][tokens[current].len() - 1],
                ));
            }

            separated.push(node.unwrap());
            left_tokens.clear();
            offset += 1;
            last_offset = offset;
        } else {
            left_tokens.push(tokens[current + offset]);
            offset += 1;
        }
    }
    if separated.len() == 0 {
        return Ok((None, 0));
    }
    let (node, node_offset) = match_all(&left_tokens, 0)?;
    if node.is_none() {
        return Ok((None, 0));
    }
    separated.push(node.unwrap());
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Expressions,
            Some(&tokens[current][0]),
            Some(separated),
        )),
        last_offset + node_offset,
    ));
}

fn match_return_and_yield<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 1 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_identifier(&tokens[current], "return") && !is_identifier(&tokens[current], "yield") {
        return Ok((None, 0));
    }
    let (guess, guess_offset) = match_all(tokens, current + 1)?;
    if guess.is_none() {
        return Ok((None, 0));
    }
    let guess = guess.unwrap();

    let node_type = if is_identifier(&tokens[current], "return") {
        ASTNodeType::Return
    } else {
        ASTNodeType::Yield
    };
    return Ok((
        Some(ASTNode::new(
            node_type,
            Some(&tokens[current][0]),
            Some(vec![guess]),
        )),
        guess_offset + 1,
    ));
}

fn match_tuple<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset = 0usize;
    let mut left_tokens = Vec::<GatheredTokens>::new();
    let mut last_offset = 0usize;
    let mut separated = Vec::<ASTNode>::new();
    while current + offset < tokens.len() {
        if is_symbol(&tokens[current + offset], ",") {
            let (node, node_offset) = match_all(&left_tokens, 0)?;
            if node.is_none() {
                return Ok((None, 0));
            }
            if node_offset != left_tokens.len() {
                return Err(ParserError::NotFullyMatched(
                    &tokens[current][0],
                    &tokens[current][tokens[current].len() - 1],
                ));
            }
            separated.push(node.unwrap());
            left_tokens.clear();
            offset += 1;
            last_offset = offset;
        } else {
            left_tokens.push(tokens[current + offset]);
            offset += 1;
        }
    }
    if separated.len() == 0 {
        return Ok((None, 0));
    }
    let (node, node_offset) = match_all(&left_tokens, 0)?;
    if node.is_none() {
        return Ok((None, 0));
    }
    separated.push(node.unwrap());
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Tuple,
            Some(&tokens[current][0]),
            Some(separated),
        )),
        last_offset + node_offset,
    ));
}

fn match_let<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], ":=") {
        // x := expression
        return Ok((None, 0));
    }

    let left_tokens = gather(&tokens[current])?;

    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let left = left.unwrap();

    if !matches!(
        left.node_type,
        ASTNodeType::Variable(_) | ASTNodeType::String(_)
    ) {
        return Err(ParserError::InvalidVariableName(&tokens[current][0]));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Let(tokens[current][0].token.to_string()),
            Some(&tokens[current][0]),
            Some(vec![right]),
        )),
        right_offset + 2,
    ));
}

fn match_assign<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    // 确保有足够的 token 来处理赋值
    if current >= tokens.len() {
        return Ok((None, 0));
    }

    // 向右搜索 = 符号
    let mut offset = 0;
    let mut left_tokens = Vec::new();

    while current + offset < tokens.len() {
        // 找到 = 符号
        if tokens[current + offset].len() == 1
            && tokens[current + offset][0].token_type == TokenType::SYMBOL
            && tokens[current + offset][0].token == "="
        {
            break;
        }

        left_tokens.push(tokens[current + offset]);
        offset += 1;
    }

    // 没找到 = 符号
    if current + offset >= tokens.len() || !is_symbol(&tokens[current + offset], "=") {
        return Ok((None, 0));
    }

    // 解析左侧表达式
    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &left_tokens[0][0],
            &left_tokens[left_tokens.len() - 1][left_tokens[left_tokens.len() - 1].len() - 1],
        ));
    }
    let left = left.unwrap();

    // 解析右侧表达式
    let (right, right_offset) = match_all(tokens, current + offset + 1)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Assign,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        offset + right_offset + 1, // +1 是因为 = 符号
    ));
}
fn match_named_to<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], "=>") {
        // x => y (x is name of y)
        return Ok((None, 0));
    }

    let left_tokens = gather(&tokens[current])?;

    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let mut left = left.unwrap();

    if let ASTNodeType::Variable(name) = left.node_type {
        left = ASTNode::new(ASTNodeType::String(name), left.token, Some(left.children));
    }

    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::NamedTo,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        right_offset + 2,
    ));
}

fn match_key_value<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], ":") {
        // x: y
        return Ok((None, 0));
    }

    let left_tokens = gather(&tokens[current])?;
    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let left = left.unwrap();

    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::KeyValue,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        right_offset + 2,
    ));
}

fn match_while<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_identifier(&tokens[current], "while") {
        // while expression body
        return Ok((None, 0));
    }

    let condition_tokens = gather(&tokens[current + 1])?;

    let (condition, condition_offset) = match_all(&condition_tokens, 0)?;
    if condition.is_none() {
        return Ok((None, 0));
    }
    if condition_offset != condition_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let condition = condition.unwrap();

    let (body, body_offset) = match_all(tokens, current + 2)?;
    if body.is_none() {
        return Ok((None, 0));
    }
    let body = body.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::While,
            Some(&tokens[current][0]),
            Some(vec![condition, body]),
        )),
        body_offset + 2,
    ));
}

fn match_if<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_identifier(&tokens[current], "if") {
        // if expression truecondition || if expression truecondition else falsecondition
        return Ok((None, 0));
    }

    let condition_tokens = gather(&tokens[current + 1])?;
    let true_condition_tokens = gather(&tokens[current + 2])?;

    let (condition, condition_offset) = match_all(&condition_tokens, 0)?;
    if condition.is_none() {
        return Ok((None, 0));
    }
    if condition_offset != condition_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let condition = condition.unwrap();

    let (true_condition, true_condition_offset) = match_all(&true_condition_tokens, 0)?;
    if true_condition.is_none() {
        return Ok((None, 0));
    }
    if true_condition_offset != true_condition_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let true_condition = true_condition.unwrap();

    if current + 3 < tokens.len() && is_identifier(&tokens[current + 3], "else") {
        let false_condition_tokens = gather(&tokens[current + 4])?;
        let (false_condition, false_condition_offset) = match_all(&false_condition_tokens, 0)?;
        if false_condition.is_none() {
            return Ok((None, 0));
        }
        if false_condition_offset != false_condition_tokens.len() {
            return Err(ParserError::NotFullyMatched(
                &tokens[current][0],
                &tokens[current][tokens[current].len() - 1],
            ));
        }
        let false_condition = false_condition.unwrap();
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::If,
                Some(&tokens[current][0]),
                Some(vec![condition, true_condition, false_condition]),
            )),
            false_condition_offset + 4,
        ));
    }
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::If,
            Some(&tokens[current][0]),
            Some(vec![condition, true_condition]),
        )),
        true_condition_offset + 2,
    ));
}

fn match_control_flow<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current >= tokens.len() {
        return Ok((None, 0));
    }
    if is_identifier(&tokens[current], "break") {
        let right_tokens = tokens[current + 1..].to_vec();
        let (right, right_offset) = match_all(&right_tokens, 0)?;
        if right.is_none() {
            return Ok((None, 0));
        }
        if right_offset != right_tokens.len() {
            return Err(ParserError::NotFullyMatched(
                &tokens[current][0],
                &tokens[current][tokens[current].len() - 1],
            ));
        }
        let right = right.unwrap();
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Break,
                Some(&tokens[current][0]),
                Some(vec![right]),
            )),
            right_offset + 1,
        ));
    } else if is_identifier(&tokens[current], "continue") {
        let right_tokens = tokens[current + 1..].to_vec();
        let (right, right_offset) = match_all(&right_tokens, 0)?;
        if right.is_none() {
            return Ok((None, 0));
        }
        if right_offset != right_tokens.len() {
            return Err(ParserError::NotFullyMatched(
                &tokens[current][0],
                &tokens[current][tokens[current].len() - 1],
            ));
        }
        let right = right.unwrap();
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Continue,
                Some(&tokens[current][0]),
                Some(vec![right]),
            )),
            right_offset + 1,
        ));
    }
    return Ok((None, 0));
}

fn match_or<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset > 0 {
        let pos = current + offset;
        if is_identifier(&tokens[pos], "or") {
            operator = Some("or");
            operator_pos = pos;
            break;
        }
        offset -= 1;
    }
    if operator.is_none() {
        return Ok((None, 0));
    }

    let left_tokens = &tokens[current..operator_pos].to_vec();
    let right_tokens = &tokens[operator_pos + 1..].to_vec();

    let (left, left_offset) = match_all(left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(right_tokens, 0)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();
    if right_offset != right_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Operation(ASTNodeOperation::Or),
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}

fn match_and<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset > 0 {
        let pos = current + offset;
        if is_identifier(&tokens[pos], "and") {
            operator = Some("and");
            operator_pos = pos;
            break;
        }
        offset -= 1;
    }
    if operator.is_none() {
        return Ok((None, 0));
    }

    let left_tokens = &tokens[current..operator_pos].to_vec();
    let right_tokens = &tokens[operator_pos + 1..].to_vec();

    let (left, left_offset) = match_all(left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(right_tokens, 0)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();
    if right_offset != right_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Operation(ASTNodeOperation::And),
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}

fn match_not<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current >= tokens.len() {
        return Ok((None, 0));
    }
    if is_identifier(&tokens[current], "not") {
        let (node, node_offset) = match_all(tokens, current + 1)?;
        if node.is_none() {
            return Ok((None, 0));
        }
        let node = node.unwrap();
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Operation(ASTNodeOperation::Not),
                Some(&tokens[current][0]),
                Some(vec![node]),
            )),
            node_offset + 1,
        ));
    }
    return Ok((None, 0));
}

// >, <, >=, <=, ==, !=
fn match_operation_compare<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset > 0 {
        let pos = current + offset;
        if is_symbol(&tokens[pos], ">")
            || is_symbol(&tokens[pos], "<")
            || is_symbol(&tokens[pos], ">=")
            || is_symbol(&tokens[pos], "<=")
            || is_symbol(&tokens[pos], "==")
            || is_symbol(&tokens[pos], "!=")
        {
            operator = Some(tokens[pos][0].token);
            operator_pos = pos;
            break;
        }
        offset -= 1;
    }
    if operator.is_none() {
        return Ok((None, 0));
    }

    let left_tokens = &tokens[current..operator_pos].to_vec();
    let right_tokens = &tokens[operator_pos + 1..].to_vec();

    let (left, left_offset) = match_all(left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(right_tokens, 0)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();
    if right_offset != right_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let operation = match operator.unwrap() {
        ">" => ASTNodeOperation::Greater,
        "<" => ASTNodeOperation::Less,
        ">=" => ASTNodeOperation::GreaterEqual,
        "<=" => ASTNodeOperation::LessEqual,
        "==" => ASTNodeOperation::Equal,
        "!=" => ASTNodeOperation::NotEqual,
        _ => unreachable!(),
    };
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Operation(operation),
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}

// +, -
fn match_operation_add_sub<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len().saturating_sub(current).saturating_sub(1);
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;

    // 从右往左查找 + 或 - 操作符
    while offset > 0 {
        let pos = current + offset;
        if is_symbol(&tokens[pos], "+") || is_symbol(&tokens[pos], "-") {
            let op_token = tokens[pos][0].token;

            // 检查是否为一元操作符
            let is_unary = if pos == current {
                true // 如果在表达式开始位置，一定是一元操作符
            } else {
                // 检查前一个token是否为运算符或括号等，表明这是一元操作符
                let prev_pos = pos - 1;
                tokens[prev_pos].len() == 1
                    && tokens[prev_pos][0].token_type == TokenType::SYMBOL
                    && vec![
                        "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "(",
                    ]
                    .contains(&tokens[prev_pos][0].token)
            };

            // 如果是一元操作符，继续向左搜索二元操作符
            if is_unary && pos > current {
                offset -= 1;
                continue;
            }

            operator = Some(op_token);
            operator_pos = pos;
            break;
        }
        offset -= 1;
    }

    if operator.is_none() {
        return Ok((None, 0)); // 没有找到操作符
    }

    let op = operator.unwrap();

    // 处理一元操作符的情况(+x, -x)
    if operator_pos == current
        || (operator_pos > current
            && tokens[operator_pos - 1].len() == 1
            && tokens[operator_pos - 1][0].token_type == TokenType::SYMBOL
            && vec![
                "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "(",
            ]
            .contains(&tokens[operator_pos - 1][0].token))
    {
        // 解析右侧表达式
        let right_tokens = &tokens[operator_pos + 1..].to_vec();
        let (right, right_offset) = match_all(right_tokens, 0)?;

        if right.is_none() {
            return Ok((None, 0));
        }

        let right = right.unwrap();
        if right_offset != right_tokens.len() {
            return Err(ParserError::NotFullyMatched(
                &tokens[current][0],
                &tokens[current][tokens[current].len() - 1],
            ));
        }

        // 确定操作类型
        let operation = if op == "+" {
            ASTNodeOperation::Add
        } else {
            ASTNodeOperation::Subtract
        };

        // 返回一元操作节点
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Operation(operation),
                Some(&tokens[operator_pos][0]),
                Some(vec![right]),
            )),
            tokens.len() - current,
        ));
    }

    // 处理二元操作符情况

    // 解析左侧表达式
    let left_tokens = &tokens[current..operator_pos].to_vec();
    let (left, left_offset) = match_all(left_tokens, 0)?;

    if left.is_none() {
        return Ok((None, 0));
    }

    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    // 解析右侧表达式
    let right_tokens = &tokens[operator_pos + 1..].to_vec();
    let (right, right_offset) = match_all(right_tokens, 0)?;

    if right.is_none() {
        return Ok((None, 0));
    }

    let right = right.unwrap();
    if right_offset != right_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    // 确定操作类型
    let operation = if op == "+" {
        ASTNodeOperation::Add
    } else {
        ASTNodeOperation::Subtract
    };

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Operation(operation),
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        tokens.len() - current, // 返回整个匹配长度
    ));
}

fn match_operation_mul_div_mod<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len().saturating_sub(current).saturating_sub(1);
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;

    // 从右往左查找 *, / 或 % 操作符
    while offset > 0 {
        let pos = current + offset;
        if is_symbol(&tokens[pos], "*")
            || is_symbol(&tokens[pos], "/")
            || is_symbol(&tokens[pos], "%")
        {
            operator = Some(tokens[pos][0].token);
            operator_pos = pos;
            break;
        }
        offset -= 1;
    }

    if operator.is_none() {
        return Ok((None, 0)); // 没有找到操作符
    }

    // 解析左侧表达式
    let left_tokens = &tokens[current..operator_pos].to_vec();
    let (left, left_offset) = match_all(left_tokens, 0)?;

    if left.is_none() {
        return Ok((None, 0));
    }

    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[operator_pos][tokens[operator_pos].len() - 1],
        ));
    }

    // 解析右侧表达式
    let right_tokens = &tokens[operator_pos + 1..].to_vec();
    let (right, right_offset) = match_all(right_tokens, 0)?;

    if right.is_none() {
        return Ok((None, 0));
    }

    let right = right.unwrap();
    if right_offset != right_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[operator_pos + 1][0],
            &tokens[tokens.len() - 1][tokens[tokens.len() - 1].len() - 1],
        ));
    }

    // 确定操作类型
    let operation = match operator.unwrap() {
        "*" => ASTNodeOperation::Multiply,
        "/" => ASTNodeOperation::Divide,
        "%" => ASTNodeOperation::Modulus,
        _ => unreachable!(),
    };

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Operation(operation),
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        tokens.len() - current, // 返回整个匹配长度
    ));
}

fn match_lambda_def<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], "->") {
        // (x, y) -> expression
        return Ok((None, 0));
    }
    let left_tokens = gather(&tokens[current])?;
    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::LambdaDef,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        right_offset + 2,
    ));
}

fn match_modifier<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 1 >= tokens.len() {
        return Ok((None, 0));
    }
    if tokens[current].len() == 1
        && vec![
            "deepcopy", "copy", "ref", "deref", "keyof", "valueof", "selfof", "assert", "import",
            "wrap", "typeof", "await", "wipe", "aliasof", "bind",
        ]
        .contains(&tokens[current][0].token)
    {
        let (node, node_offset) = match_all(tokens, current + 1)?;
        if node.is_none() {
            return Ok((None, 0));
        }
        let node = node.unwrap();

        let modifier = match tokens[current][0].token {
            "deepcopy" => ASTNodeModifier::DeepCopy,
            "copy" => ASTNodeModifier::Copy,
            "ref" => ASTNodeModifier::Ref,
            "deref" => ASTNodeModifier::Deref,
            "keyof" => ASTNodeModifier::KeyOf,
            "valueof" => ASTNodeModifier::ValueOf,
            "selfof" => ASTNodeModifier::SelfOf,
            "assert" => ASTNodeModifier::Assert,
            "import" => ASTNodeModifier::Import,
            "wrap" => ASTNodeModifier::Wrap,
            "typeof" => ASTNodeModifier::TypeOf,
            "await" => ASTNodeModifier::Await,
            "wipe" => ASTNodeModifier::Wipe,
            "aliasof" => ASTNodeModifier::AliasOf,
            "bind" => ASTNodeModifier::BindSelf,
            _ => return Ok((None, 0)),
        };
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Modifier(modifier),
                Some(&tokens[current][0]),
                Some(vec![node]),
            )),
            node_offset + 1,
        ));
    }
    return Ok((None, 0));
}

fn match_named_to_null<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    // expr ?
    if current + 1 >= tokens.len() {
        return Ok((None, 0));
    }
    if is_symbol(&tokens[tokens.len() - 1], "?") {
        let left_tokens = tokens[..tokens.len() - 1].to_vec();
        let (node, node_offset) = match_all(&left_tokens, 0 )?;
        if node.is_none() {
            return Ok((None, 0));
        }
        if node_offset != left_tokens.len() {
            return Err(ParserError::NotFullyMatched(
                &tokens[current][0],
                &tokens[current][tokens[current].len() - 1],
            ));
        }
        let mut node = node.unwrap();
        if let ASTNodeType::Variable(name) = node.node_type {
            node = ASTNode::new(ASTNodeType::String(name), node.token, Some(node.children));
        }
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::NamedTo,
                Some(&tokens[current][0]),
                Some(vec![node, ASTNode::new(ASTNodeType::Null, None, None)]),
            )),
            node_offset + 1,
        ));
    }
    return Ok((None, 0));
}

fn match_alias<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }

    if !is_symbol(&tokens[current + 1], "::") {
        return Ok((None, 0));
    }

    let type_tokens = gather(&tokens[current])?;
    let (type_node, type_offset) = match_all(&type_tokens, 0)?;

    if type_node.is_none() {
        return Ok((None, 0));
    }

    let type_node = type_node.unwrap();
    if type_offset != type_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    let type_name = match &type_node.node_type {
        ASTNodeType::Variable(name) => name.clone(),
        _ => {
            return Err(ParserError::InvalidSyntax(&tokens[current][0]));
        }
    };

    // 解析右侧值表达式
    let (value_node, value_offset) = match_all(tokens, current + 2)?;
    if value_node.is_none() {
        return Ok((None, 0));
    }
    let value_node = value_node.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Alias(type_name),
            Some(&tokens[current][0]),
            Some(vec![value_node]),
        )),
        value_offset + 2,
    ));
}

fn match_member_access_and_call<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut access_type = Option::<&str>::None;
    let mut access_pos: usize = 0;

    // 从右往左搜索访问操作符
    while offset > 0 {
        let pos = current + offset;

        // 检查是否为索引访问 obj[idx]
        if is_square_bracket(&tokens[pos]) {
            access_type = Some("[]");
            access_pos = pos;
            break;
        }
        // 检查是否为属性访问 obj.prop
        else if is_symbol(&tokens[pos], ".") {
            access_type = Some(".");
            access_pos = pos;
            break;
        }
        // 检查是否为函数调用 func(args)
        else if is_bracket(&tokens[pos]) {
            access_type = Some("()");
            access_pos = pos;
            break;
        }

        offset -= 1;
    }

    if access_type.is_none() {
        return Ok((None, 0)); // 没有找到访问操作符
    }

    match access_type.unwrap() {
        // 处理索引访问 obj[idx]
        "[]" => {
            // 解析左侧表达式（被访问的对象或被调用的函数）
            let left_tokens = &tokens[current..access_pos].to_vec();
            if left_tokens.len() == 0 {
                return Ok((None, 0));
            }

            let (left, left_offset) = match_all(left_tokens, 0)?;
            if left.is_none() {
                return Ok((None, 0));
            }

            let left = left.unwrap();
            if left_offset != left_tokens.len() {
                return Err(ParserError::NotFullyMatched(
                    &tokens[current][0],
                    &tokens[current][tokens[current].len() - 1],
                ));
            }
            // 解包索引括号中的内容
            let index_tokens = unwrap_brace(&tokens[access_pos])?;
            let gathered_index = gather(index_tokens)?;

            let (index_node, _) = match_all(&gathered_index, 0)?;
            if index_node.is_none() {
                return Ok((None, 0));
            }

            let index_node = index_node.unwrap();

            return Ok((
                Some(ASTNode::new(
                    ASTNodeType::IndexOf,
                    Some(&tokens[current][0]),
                    Some(vec![left, index_node]),
                )),
                (access_pos - current) + 1,
            ));
        }

        // 处理属性访问 obj.prop
        "." => {
            // 解析左侧表达式（被访问的对象或被调用的函数）
            let left_tokens = &tokens[current..access_pos].to_vec();
            if left_tokens.len() == 0 {
                return Ok((None, 0));
            }

            let (left, left_offset) = match_all(left_tokens, 0)?;
            if left.is_none() {
                return Ok((None, 0));
            }

            let left = left.unwrap();
            if left_offset != left_tokens.len() {
                return Err(ParserError::NotFullyMatched(
                    &tokens[current][0],
                    &tokens[current][tokens[current].len() - 1],
                ));
            }
            if access_pos + 1 >= tokens.len() {
                return Ok((None, 0));
            }

            // 获取属性名称
            let right_tokens = &tokens[access_pos + 1..].to_vec();
            let (right, right_offset) = match_all(right_tokens, 0)?;
            if right.is_none() {
                return Ok((None, 0));
            }

            let mut right = right.unwrap();

            // 如果右侧是变量，将其视为属性名
            if let ASTNodeType::Variable(var_name) = right.node_type {
                right = ASTNode::new(
                    ASTNodeType::String(var_name),
                    right.token,
                    Some(right.children),
                );
                return Ok((
                    Some(ASTNode::new(
                        ASTNodeType::GetAttr,
                        Some(&tokens[current][0]),
                        Some(vec![left, right]),
                    )),
                    (access_pos - current) + 1 + right_offset,
                ));
            }

            return Ok((
                Some(ASTNode::new(
                    ASTNodeType::GetAttr,
                    Some(&tokens[current][0]),
                    Some(vec![left, right]),
                )),
                (access_pos - current) + 1 + right_offset,
            ));
        }

        // 处理函数调用 func(args)
        "()" => {
            // 解析左侧表达式（被访问的对象或被调用的函数）
            let mut left_tokens = tokens[current..access_pos].to_vec();
            if left_tokens.len() == 0 {
                return Ok((None, 0));
            }

            let is_async = if left_tokens[0].len() == 1 && left_tokens[0][0].token == "async" {
                left_tokens = left_tokens[1..].to_vec();
                true
            } else {
                false
            };
            let (left, left_offset) = match_all(&left_tokens, 0)?;
            if left.is_none() {
                return Ok((None, 0));
            }

            let left = left.unwrap();
            if left_offset != left_tokens.len() {
                return Err(ParserError::NotFullyMatched(
                    &tokens[current][0],
                    &tokens[current][tokens[current].len() - 1],
                ));
            }

            // 解包括号中的参数
            let args_tokens = unwrap_brace(&tokens[access_pos])?;
            let gathered_args = gather(args_tokens)?;

            // 处理有参数情况
            let (args_node, _) = match_all(&gathered_args, 0)?;
            if args_node.is_none() {
                return Ok((None, 0));
            }

            let args_node = args_node.unwrap();

            // 如果不是元组类型，将其包装为元组
            let args = if args_node.node_type != ASTNodeType::Tuple {
                ASTNode::new(
                    ASTNodeType::Tuple,
                    Some(&tokens[access_pos][0]),
                    Some(vec![args_node]),
                )
            } else {
                args_node
            };

            return Ok((
                Some(ASTNode::new(
                    if is_async {
                        ASTNodeType::AsyncLambdaCall
                    } else {
                        ASTNodeType::LambdaCall
                    },
                    Some(&tokens[current][0]),
                    Some(vec![left, args]),
                )),
                (access_pos - current) + 1,
            ));
        }

        _ => unreachable!(),
    }
}

fn match_range<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], "..") {
        // x..y
        return Ok((None, 0));
    }

    let left_tokens = gather(&tokens[current])?;
    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Range,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        right_offset + 2,
    ));
}

fn match_in<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_identifier(&tokens[current + 1], "in") {
        return Ok((None, 0));
    }

    let left_tokens = gather(&tokens[current])?;
    let (left, left_offset) = match_all(&left_tokens, 0)?;
    if left.is_none() {
        return Ok((None, 0));
    }
    let left = left.unwrap();
    if left_offset != left_tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }
    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::In,
            Some(&tokens[current][0]),
            Some(vec![left, right]),
        )),
        right_offset + 2,
    ));
}

fn match_variable<'t>(
    tokens: &Vec<GatheredTokens<'t>>,
    current: usize,
) -> Result<(Option<ASTNode<'t>>, usize), ParserError<'t>> {
    if current >= tokens.len() {
        return Ok((None, 0));
    }

    // 匹配括号内容（元组）
    if is_bracket(&tokens[current]) || is_square_bracket(&tokens[current]) {
        let inner_tokens = unwrap_brace(&tokens[current])?;

        // 处理空元组 ()
        if inner_tokens.len() == 0 {
            return Ok((
                Some(ASTNode::new(
                    ASTNodeType::Tuple,
                    Some(&tokens[current][0]),
                    None,
                )),
                1,
            ));
        }

        let gathered_inner = gather(inner_tokens)?;
        let (node, _) = match_all(&gathered_inner, 0)?;
        if node.is_none() {
            return Ok((None, 0));
        }

        return Ok((Some(node.unwrap()), 1));
    }

    // 匹配函数体 {...}
    if is_brace(&tokens[current]) {
        let body_tokens = unwrap_brace(&tokens[current])?;
        let gathered_body = gather(body_tokens)?;
        let (body, _) = match_all(&gathered_body, 0)?;

        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Body,
                Some(&tokens[current][0]),
                body.map(|b| vec![b]),
            )),
            1,
        ));
    }

    // 匹配字符串常量
    if tokens[current].len() == 1 && tokens[current][0].token_type == TokenType::STRING {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::String(tokens[current][0].token.to_string()),
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }

    // 匹配数字常量
    if tokens[current].len() == 1 && tokens[current][0].token_type == TokenType::NUMBER {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Number(tokens[current][0].token.to_string()),
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }

    // 匹配布尔值（true）
    if is_identifier(&tokens[current], "true") {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Boolean(String::from("true")),
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }

    // 匹配布尔值（false）
    if is_identifier(&tokens[current], "false") {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Boolean(String::from("false")),
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }

    // 匹配空值（null）
    if is_identifier(&tokens[current], "null") {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Null,
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }

    // 匹配普通变量名
    if tokens[current].len() == 1 && tokens[current][0].token_type == TokenType::IDENTIFIER {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Variable(tokens[current][0].token.to_string()),
                Some(&tokens[current][0]),
                None,
            )),
            1,
        ));
    }
    return Ok((None, 0));
}
