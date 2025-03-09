use std::vec;

use crate::{lexer, Token, TokenType};

#[derive(Debug)]
pub enum ParserError<'T> {
    UnexpectedToken(&'T Token<'T>),                     // Token
    UnmatchedParenthesis(&'T Token<'T>, &'T Token<'T>), // (opening, closing)
    InvalidSyntax(&'T Token<'T>),
    NotFullyMatched(&'T Token<'T>, &'T Token<'T>),
}

type GatheredTokens<'T> = Vec<&'T Token<'T>>;

fn get_next_tokens<'a>(
    tokens: &'a Vec<Token>,
    current: usize,
) -> Result<Vec<&'a Token<'a>>, ParserError<'a>> {
    let mut stack = Vec::<(&str, usize)>::new();
    let mut next_tokens = Vec::<&'a Token>::new();
    let mut current = current;
    if current >= (*tokens).len() {
        return Ok(next_tokens);
    }
    loop {
        if vec!["{", "[", "("].contains(&tokens[current].token)
            && tokens[current].token_type == TokenType::SYMBOL
        {
            stack.push((tokens[current].token, current));
            next_tokens.push(&tokens[current]);
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

            next_tokens.push(&tokens[current]);
        } else {
            next_tokens.push(&tokens[current]);
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
    return Ok(next_tokens);
}

fn gather<'T>(tokens: &'T Vec<Token<'T>>) -> Result<Vec<GatheredTokens<'T>>, ParserError<'T>> {
    let mut current = 0;
    let mut result = Vec::<GatheredTokens>::new();
    while current < tokens.len() {
        let next_tokens = get_next_tokens(tokens, current)?;
        current += next_tokens.len();
        result.push(next_tokens);
    }
    Ok(result)
}

#[derive(Debug)]
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
    NamedAs,     // x => y (x is name of y)
    Break,       // break
    Continue,    // continue
}

#[derive(Debug)]
pub struct ASTNode<'T> {
    pub node_type: ASTNodeType,     // Type of the node
    pub token: &'T Token<'T>,       // Token associated with the node
    pub children: Vec<ASTNode<'T>>, // Children of the node
}

impl ASTNode<'_> {
    pub fn new<'T>(
        node_type: ASTNodeType,
        token: &'T Token,
        children: Option<Vec<ASTNode<'T>>>,
    ) -> ASTNode<'T> {
        ASTNode {
            node_type,
            token,
            children: match children {
                Some(children) => children,
                None => Vec::new(),
            },
        }
    }
}

trait MatcherFunction<'a>:
    Fn(&Vec<GatheredTokens<'a>>, usize) -> Result<Option<(ASTNode<'a>, usize)>, ParserError<'a>>
    + std::fmt::Debug
{
}
impl<'a, F> MatcherFunction<'a> for F where
    F: Fn(&Vec<GatheredTokens<'a>>, usize) -> Result<Option<(ASTNode<'a>, usize)>, ParserError<'a>>
        + std::fmt::Debug
{
}

type MatcherFn<'a> = Box<dyn MatcherFunction<'a> + 'a>;
#[derive(Debug)]
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
    ) -> Result<Option<(ASTNode<'a>, usize)>, ParserError<'a>> {
        for matcher in &self.matchers {
            if let Ok(Some(node)) = matcher(tokens, current) {
                return Ok(Some(node));
            }
        }
        Ok(None)
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

fn match_expressions<'T>(
    tokens: &Vec<&GatheredTokens<'T>>,
    current: usize,
) -> Result<(Option<ASTNode<'T>>, usize), ParserError<'T>> {
    let mut offset = 0usize;
    let mut left_tokens = Vec::<&GatheredTokens>::new();
    let mut last_offset = 0usize;
    let mut separated = Vec::<ASTNode>::new();
    while current + offset < tokens.len() {
        if is_symbol(&tokens[current + offset], ";") {
            let (node, node_offset) = match_expressions(tokens, current + offset + 1)?;
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
            left_tokens.push(&tokens[current + offset]);
            offset += 1;
        }
    }
    if separated.len() == 0 {
        return Ok((None, 0));
    }
    let (node, node_offset) = match_expressions(&left_tokens, 0)?;
    if node.is_none() {
        return Ok((None, 0));
    }
    separated.push(node.unwrap());
    return Ok((
        Some(ASTNode::new(
            ASTNodeType::Expressions,
            &tokens[current][0],
            Some(separated),
        )),
        current + node_offset,
    ));
}
