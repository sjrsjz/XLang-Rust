
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
    let mut gc = GCSystem::new();
    let mut A_i32 = gc.new_object(GCInteger::new(10));
    let mut B_i32 = gc.new_object(GCInteger::new(20));
    let mut C_i32 = gc.new_object(GCInteger::new(30));
    
    println!("Before offline");
    gc.debug_print();

    A_i32.offline();

    println!("After offline");
    gc.debug_print();

    gc.collect();

    println!("After collect");
    gc.debug_print();

    println!("B: {:?}", GCSystem::as_type::<GCInteger>(&B_i32).value);
    println!("C: {:?}", GCSystem::as_type::<GCInteger>(&C_i32).value);

    B_i32.offline();
    C_i32.offline();

    println!("After offline All");
    gc.debug_print();

    gc.collect();

    println!("After collect All");
    gc.debug_print();
    
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