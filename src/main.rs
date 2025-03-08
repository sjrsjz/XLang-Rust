
mod parser;
use self::parser::lexer::{
    TokenType,
    lexer,
    Token
};
fn main() {
    let code = r#"
        let x = 5; // This is a comment
        let y = 10;
        let sum = x + y;
        println!("Sum: {}", sum);"#;
    let lexer = lexer::reject_comment(lexer::tokenize(code));
    for token in lexer {
        print!("{:?} ", token.to_string());
    }
}