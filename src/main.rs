mod parser;
pub mod vm;
use self::parser::ast::build_ast;
use self::parser::ast::ast_token_stream;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::GCSystem;

use self::vm::ir_generator::ir_generator;
use self::vm::ir::Functions;


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

print(factorial(5));

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

    let ir = ir_generator.generate(&ast.unwrap());
    if ir.is_err() {
        println!("Error: {:?}", ir.err().unwrap());
        return;
    }
    functions.append("__main__".to_string(), ir.unwrap());
    
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
