
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
Z := (f => (x => null) -> { return x(x); }) -> {
    return f((x => null, f => f) -> {
        return f(Z(f))(x);
    });
};


factorial := Z((f => null) -> {
    return (n => 0, f => f) -> {
        if (n <= 1) {
            return 1;
        } else {
            return n * f(n - 1);
        };
    };
});

print(factorial(5));

"#;
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
    println!("\n\nAST:\n");

    ast.unwrap().formatted_print(0);
    
}