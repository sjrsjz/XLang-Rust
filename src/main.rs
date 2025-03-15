mod parser;
pub mod vm;
use vm::executor;
use vm::executor::variable::VMInstructions;
use vm::executor::variable::VMLambda;
use vm::executor::variable::VMTuple;
use vm::ir::IR;

use self::parser::ast::build_ast;
use self::parser::ast::ast_token_stream;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::GCSystem;

use self::vm::ir_generator::ir_generator;
use self::vm::ir::Functions;
use self::vm::executor::vm::*;
use self::vm::executor::variable::*;


fn main() {

    let code = r#"
i:=0;

while (i < 10) {
    i = i + 1;
    print(i);
};


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
    let mut ir = ir.unwrap();
    ir.push(IR::Return);
    functions.append("__main__".to_string(), ir);
    
    let (built_ins, ips) = functions.build_instructions();
    println!("\n\nBuilt Ins:\n");
    for ir in &built_ins {
        println!("{:?}", ir);
    }
    println!("\n\nFunction IPs:\n");
    for (name, ip) in &ips {
        println!("{}: {:?}", name, ip);
    }

    let mut executor = IRExecutor::new(Some(code.to_string()));
    let mut gc_system = GCSystem::new();

    let default_args_tuple = gc_system.new_object(VMTuple::new(vec![]));
    let lambda_instructions = gc_system.new_object(VMInstructions::new(built_ins, ips));
    
    let main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        default_args_tuple.clone(),
        None,
        lambda_instructions.clone(),
    ));
    default_args_tuple.offline();
    lambda_instructions.offline();

    let result = executor.execute(main_lambda, &mut gc_system);
    if result.is_err() {
        println!("Error: {:?}", result.err().unwrap());
        gc_system.debug_print();
        return;
    }
    let result = result.unwrap();
    let repr = try_repr_vmobject(result.clone());
    if repr.is_ok(){
        println!("Result: {:?}", repr.unwrap());
    } else {
        println!("Error: {:?}", repr.err().unwrap());
    }
    print!("\n\nResult GCRef: {:?}\n", result);
    result.offline();
    gc_system.collect();
    println!("Existing GCRef: {:?}", gc_system.count());
    gc_system.print_reference_graph();


}
