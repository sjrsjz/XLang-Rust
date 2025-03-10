
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
        lazy := (computation => null) -> {
    result := null;
    evaluated := false;
    
    return (evaluated => evaluated,
            result => result,
            computation => computation) -> {
        if (evaluated == false) {
            result = computation();
            evaluated = true;
        };
        return result;
    };
};

expensiveComputation := lazy(() -> {
    print("Computing...");
    return 42;
});


print(expensiveComputation()); 

print(expensiveComputation()); "#;
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