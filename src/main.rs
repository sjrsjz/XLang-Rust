
mod parser;
use self::parser::lexer::{
    TokenType,
    lexer,
    Token
};
use self::parser::ast::build_ast;
use self::parser::ast::ASTTokenStream;
fn main() {
    let code = r#"
print(1)"#;
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    for token in &tokens {
        print!("{:?} ", token.to_string());
    }

    let gathered = ASTTokenStream::from_stream(&tokens);
    let ast = build_ast(&gathered);
    if ast.is_err() {
        println!("");
        println!("Error: {:?}", ast.err());
        return;
    }
    println!("\n\nAST:");
    
}