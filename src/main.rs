mod parser;
pub mod vm;
use self::parser::ast::build_ast;
use self::parser::ast::ast_token_stream;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::GCSystem;
use self::vm::gc::variable::GCInteger;

use self::vm::ir_generator::ir_generator;
use self::vm::ir::Functions;



fn gc_test() {
    let mut gc = GCSystem::new();
    let A_i32 = gc.new_object(GCInteger::new(10));
    let B_i32 = gc.new_object(GCInteger::new(20));
    let C_i32 = gc.new_object(GCInteger::new(30));

    println!("Before offline");
    gc.debug_print();

    A_i32.offline();

    println!("After offline");
    gc.debug_print();

    gc.collect();

    println!("After collect");
    gc.debug_print();

    println!("B: {:?}", B_i32.as_type::<GCInteger>().value);
    println!("C: {:?}", C_i32.as_type::<GCInteger>().value);

    B_i32.offline();
    C_i32.offline();

    println!("After offline All");
    gc.debug_print();

    gc.collect();

    println!("After collect All");
    gc.debug_print();
}

fn main() {

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

    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = build_ast(&gathered);
    if ast.is_err() {
        println!("");
        let err_token = ast.err().unwrap();
        println!("Error token: {:?}", err_token.format(&tokens));
        return;
    }
    println!("\n\nAST:\n");

    ast.as_ref().unwrap().formatted_print(0);

    let namespace = ir_generator::NameSpace::new("Main".to_string(), None);
    let mut functions = Functions::new();
    let mut ir_generator = ir_generator::IRGenerator::new(&mut functions, namespace);

    let ir = ir_generator.generate_without_redirect(&ast.unwrap());
    println!("\n\nIR:\n");
    for ir in &ir {
        println!("{:?}", ir);
    }

    let (built_ins, ips) = functions.build_instructions();
    println!("\n\nBuilt Ins:\n");
    for ir in &built_ins {
        println!("{:?}", ir);
    }
    println!("\n\nFunction IPs:\n");
    for (name, ip) in &ips {
        println!("{}: {:?}", name, ip);
    }


}
