
mod parser;
pub mod vm;
use self::parser::lexer::{
    TokenType,
    lexer,
    Token
};
use self::parser::ast::build_ast;
use self::parser::ast::ASTTokenStream;


use self::vm::gc::gc::{GCObject, GCTraceable,GCSystem};
use self::vm::gc::variable::GCInteger;

fn gc_test() {

}


fn main() {

    gc_test();
    return;

    let code = r#"


factorial := Z((f => null) -> {
    return (n => 0, f => f) -> {
        if (n <= 1) {
            return 1;
        } else {
            return n * f(n - 1);
        };
    };
});

"#;
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    for token in &tokens {
        print!("{:?} ", token.to_string());
    }

    let gathered = ASTTokenStream::from_stream(&tokens);
    let ast = build_ast(&gathered);
    if ast.is_err() {
        println!("");
        let err_token = ast.err().unwrap();
        println!("Error token: {:?}", err_token.format(&tokens));
        return;
    }
    println!("\n\nAST:\n");

    ast.unwrap().formatted_print(0);
    
}