use std::{fmt::Debug, vec};

use crate::{lexer, Token, TokenType};

#[derive(Debug)]
pub enum ParserError<'T> {
    UnexpectedToken(&'T Token<'T>),                     // Token
    UnmatchedParenthesis(&'T Token<'T>, &'T Token<'T>), // (opening, closing)
    InvalidSyntax(&'T Token<'T>),
    NotFullyMatched(&'T Token<'T>, &'T Token<'T>),
    InvalidVariableName(&'T Token<'T>),
}
type TokenStream<'T> = Vec<Token<'T>>;
type GatheredTokens<'T> = &'T [Token<'T>];

fn get_next_tokens<'a>(
    tokens: GatheredTokens<'a>,
    current: usize,
) -> Result<GatheredTokens<'a>, ParserError<'a>> {
    let mut stack = Vec::<(&str, usize)>::new();
    let mut next_tokens_end = 0usize;
    let mut current = current;
    if current >= (*tokens).len() {
        return Ok(&[]);
    }
    loop {
        if vec!["{", "[", "("].contains(&tokens[current].token)
            && tokens[current].token_type == TokenType::SYMBOL
        {
            stack.push((tokens[current].token, current));
            next_tokens_end += 1;
        } else if vec!["}", "]", ")"].contains(&tokens[current].token)
            && tokens[current].token_type == TokenType::SYMBOL
        {
            if stack.len() == 0 {
                break;
            }
            let (last, last_position) = stack.pop().unwrap();
            if (last == "{" && tokens[current].token != "}")
                || (last == "[" && tokens[current].token != "]")
                || (last == "(" && tokens[current].token != ")")
            {
                return Err(ParserError::UnmatchedParenthesis(
                    &tokens[last_position],
                    &tokens[current],
                ));
            }

            next_tokens_end += 1;
        } else {
            next_tokens_end += 1;
        }
        current += 1;
        if current >= (*tokens).len() || stack.len() == 0 {
            break;
        }
    }
    if stack.len() > 0 {
        let (last, last_position) = stack.pop().unwrap();
        return Err(ParserError::UnmatchedParenthesis(
            &tokens[last_position],
            &tokens[current],
        ));
    }
    return Ok(&tokens[current..next_tokens_end]);
}

fn gather<'T>(tokens: GatheredTokens<'T>) -> Result<Vec<GatheredTokens<'T>>, ParserError<'T>> {
    let mut current = 0;
    let mut result = Vec::<GatheredTokens>::new();
    while current < tokens.len() {
        let next_tokens = get_next_tokens(tokens, current)?;
        current += next_tokens.len();
        result.push(next_tokens);
    }
    Ok(result)
}

#[derive(Debug, PartialEq)]
pub enum ASTNodeType {
    None,        // No expression
    Null,        // Null
    String,      // String
    Boolen,      // Boolean
    Number,      // Number (Integer, Float)
    Variable,    // Variable
    Let,         // x := expression
    Body,        // {...}
    Assign,      // x = expression
    LambdaDef,   // tuple -> body
    Expressions, // expression1; expression2; ...
    LambdaCall,  // x (tuple)
    Operation,   // x + y, x - y, x * y, x / y ...
    Tuple,       // x, y, z, ...
    KeyValue,    // x: y
    IndexOf,     // x[y]
    GetAttr,     // x.y
    Return,      // return expression
    If,          // if expression truecondition || if expression truecondition else falsecondition
    While,       // while expression body
    Modifier,    // modifier expression
    NamedTo,     // x => y (x is name of y)
    Break,       // break
    Continue,    // continue
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum ASTNodeAdditionalInfo {
    ASTNodeOperation(ASTNodeOperation),
}

#[derive(Debug)]
pub struct ASTNode<'T> {
    pub node_type: ASTNodeType,                        // Type of the node
    pub token: &'T Token<'T>,                          // Token associated with the node
    pub children: Vec<ASTNode<'T>>,                    // Children of the node
    pub addtional_info: Option<ASTNodeAdditionalInfo>, // Additional information about the node
}

impl ASTNode<'_> {
    pub fn new<'T>(
        node_type: ASTNodeType,
        token: &'T Token,
        children: Option<Vec<ASTNode<'T>>>,
        addtional_info: Option<ASTNodeAdditionalInfo>,
    ) -> ASTNode<'T> {
        ASTNode {
            node_type,
            token,
            children: match children {
                Some(children) => children,
                None => Vec::new(),
            },
            addtional_info: match addtional_info {
                Some(addtional_info) => Some(addtional_info),
                None => None,
            },
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
        for matcher in &self.matchers {
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

pub fn build_ast<'a>(tokens: GatheredTokens<'a>) -> Result<ASTNode<'a>, ParserError<'a>> {
    let (matched, offset) = match_all(&(gather(tokens)?), 0)?;
    if matched.is_none() {
        return Err(ParserError::InvalidSyntax(&tokens[0]));
    }
    let matched = matched.unwrap();
    if offset != tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[0],
            &tokens[tokens.len() - 1],
        ));
    }
    return Ok(matched);
}

fn match_all<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut node_matcher = NodeMatcher::new();
    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
            match_expressions(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
            match_return(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
            match_tuple(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
            match_let(tokens, current)
        },
    ));

    node_matcher.add_matcher(Box::new(
        |tokens, current| -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
            match_assign(tokens, current)
        },
    ));

    return node_matcher.match_node(tokens, current);
}

fn match_expressions<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset = 0usize;
    let mut left_tokens = Vec::<GatheredTokens>::new();
    let mut last_offset = 0usize;
    let mut separated = Vec::<ASTNode>::new();
    while current + offset < tokens.len() {
        if is_symbol(&tokens[current + offset], ";") {
            let (node, node_offset) = match_all(tokens, current + offset + 1)?;
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
            &tokens[current][0],
            Some(separated),
            None,
        )),
        last_offset + node_offset,
    ));
}

fn match_return<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    if current + 1 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_identifier(&tokens[current], "return") {
        return Ok((None, 0));
    }
    let (guess, guess_offset) = match_all(tokens, current + 1)?;
    if guess.is_none() {
        return Ok((None, 0));
    }
    let guess = guess.unwrap();
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Return,
            &tokens[current][0],
            Some(vec![guess]),
            None,
        )),
        guess_offset + 1,
    ));
}

fn match_tuple<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset = 0usize;
    let mut left_tokens = Vec::<GatheredTokens>::new();
    let mut last_offset = 0usize;
    let mut separated = Vec::<ASTNode>::new();
    while current + offset < tokens.len() {
        if is_symbol(&tokens[current + offset], ",") {
            let (node, node_offset) = match_all(tokens, current + offset + 1)?;
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
            &tokens[current][0],
            Some(separated),
            None,
        )),
        last_offset + node_offset,
    ));
}

fn match_let<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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

    if (left.node_type != ASTNodeType::Variable && left.node_type != ASTNodeType::String) {
        return Err(ParserError::InvalidVariableName(&tokens[current][0]));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Let,
            &tokens[current][0],
            Some(vec![left, right]),
            None,
        )),
        right_offset + 2,
    ));
}

fn match_assign<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    if current + 2 >= tokens.len() {
        return Ok((None, 0));
    }
    if !is_symbol(&tokens[current + 1], "=") {
        // x = expression
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

    if (left.node_type != ASTNodeType::Variable && left.node_type != ASTNodeType::String) {
        return Err(ParserError::InvalidVariableName(&tokens[current][0]));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Assign,
            &tokens[current][0],
            Some(vec![left, right]),
            None,
        )),
        right_offset + 2,
    ));
}

fn match_named_to<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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
    let left = left.unwrap();

    let (right, right_offset) = match_all(tokens, current + 2)?;
    if right.is_none() {
        return Ok((None, 0));
    }
    let right = right.unwrap();

    if right_offset != tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::NamedTo,
            &tokens[current][0],
            Some(vec![left, right]),
            None,
        )),
        right_offset + 2,
    ));
}

fn match_key_value<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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

    if right_offset != tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::KeyValue,
            &tokens[current][0],
            Some(vec![left, right]),
            None,
        )),
        right_offset + 2,
    ));
}

fn match_while<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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

    if body_offset != tokens.len() {
        return Err(ParserError::NotFullyMatched(
            &tokens[current][0],
            &tokens[current][tokens[current].len() - 1],
        ));
    }

    return Ok((
        Some(ASTNode::new(
            ASTNodeType::While,
            &tokens[current][0],
            Some(vec![condition, body]),
            None,
        )),
        body_offset + 2,
    ));
}

fn match_if<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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
                &tokens[current][0],
                Some(vec![condition, true_condition, false_condition]),
                None,
            )),
            false_condition_offset + 4,
        ));
    }
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::If,
            &tokens[current][0],
            Some(vec![condition, true_condition]),
            None,
        )),
        true_condition_offset + 2,
    ));
}

fn match_control_flow<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    if current >= tokens.len() {
        return Ok((None, 0));
    }
    if is_identifier(&tokens[current], "break") {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Break,
                &tokens[current][0],
                None,
                None,
            )),
            1,
        ));
    } else if is_identifier(&tokens[current], "continue") {
        return Ok((
            Some(ASTNode::new(
                ASTNodeType::Continue,
                &tokens[current][0],
                None,
                None,
            )),
            1,
        ));
    }
    return Ok((None, 0));
}

fn match_or<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset >= 0 {
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
            ASTNodeType::Operation,
            &tokens[current][0],
            Some(vec![left, right]),
            Some(
                ASTNodeAdditionalInfo::ASTNodeOperation(ASTNodeOperation::Or)
            ),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}



fn match_and<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset >= 0 {
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
            ASTNodeType::Operation,
            &tokens[current][0],
            Some(vec![left, right]),
            Some(
                ASTNodeAdditionalInfo::ASTNodeOperation(ASTNodeOperation::And)
            ),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}

fn match_not<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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
                ASTNodeType::Operation,
                &tokens[current][0],
                Some(vec![node]),
                Some(
                    ASTNodeAdditionalInfo::ASTNodeOperation(ASTNodeOperation::Not)
                ),
            )),
            node_offset + 1,
        ));
    }
    return Ok((None, 0));
}

// >, <, >=, <=, ==, !=
fn match_operation_compare<'T>(
    tokens: &Vec<GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset: usize = tokens.len() - current - 1;
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    while offset >= 0 {
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
            ASTNodeType::Operation,
            &tokens[current][0],
            Some(vec![left, right]),
            Some(ASTNodeAdditionalInfo::ASTNodeOperation(operation)),
        )),
        tokens.len() - current, // return full length of the tokens
    ));
}


// +, -
fn match_operation_add_sub<'T>(
    tokens: &Vec<GatheredTokens<'T>>, 
    current: usize
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
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
                tokens[prev_pos].len() == 1 && tokens[prev_pos][0].token_type == TokenType::SYMBOL &&
                vec!["+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "("].contains(&tokens[prev_pos][0].token)
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
    if operator_pos == current || 
       (operator_pos > current && 
        tokens[operator_pos - 1].len() == 1 &&
        tokens[operator_pos - 1][0].token_type == TokenType::SYMBOL &&
        vec!["+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "("].contains(&tokens[operator_pos - 1][0].token)) {
        
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
                &tokens[current][tokens[current].len() - 1]
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
                ASTNodeType::Operation,
                &tokens[operator_pos][0],
                Some(vec![right]),
                Some(ASTNodeAdditionalInfo::ASTNodeOperation(operation))
            )),
            tokens.len() - current
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
            &tokens[current][tokens[current].len() - 1]
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
            &tokens[current][tokens[current].len() - 1]
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
            ASTNodeType::Operation,
            &tokens[current][0],
            Some(vec![left, right]),
            Some(ASTNodeAdditionalInfo::ASTNodeOperation(operation))
        )),
        tokens.len() - current // 返回整个匹配长度
    ));
}


fn match_operation_mul_div_mod<'T>(
    tokens: &Vec<GatheredTokens<'T>>, 
    current: usize
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset: usize = tokens.len().saturating_sub(current).saturating_sub(1);
    let mut operator = Option::<&str>::None;
    let mut operator_pos: usize = 0;
    
    // 从右往左查找 *, / 或 % 操作符
    while offset > 0 {
        let pos = current + offset;
        if is_symbol(&tokens[pos], "*") || 
           is_symbol(&tokens[pos], "/") || 
           is_symbol(&tokens[pos], "%") {
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
            &tokens[current][tokens[current].len() - 1]
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
            &tokens[current][tokens[current].len() - 1]
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
            ASTNodeType::Operation,
            &tokens[current][0],
            Some(vec![left, right]),
            Some(ASTNodeAdditionalInfo::ASTNodeOperation(operation))
        )),
        tokens.len() - current // 返回整个匹配长度
    ));
}