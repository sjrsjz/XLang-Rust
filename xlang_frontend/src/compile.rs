use colored::Colorize;

use crate::dir_stack::DirStack;
use crate::parser::lexer::lexer;
use xlang_vm_core::instruction_set::VMInstructionPackage;
use xlang_vm_core::ir::Functions;
use xlang_vm_core::ir::IRPackage;
use crate::ir_generator::ir_generator;
use crate::parser::analyzer::analyze_ast;
use xlang_vm_core::ir::DebugInfo;
use xlang_vm_core::ir::IR;
use crate::parser::ast::ast_token_stream;
use crate::parser::ast::build_ast;
use xlang_vm_core::ir_translator::IRTranslator;



// Compile code and generate intermediate representation
pub fn build_code(code: &str, dir_stack: &mut DirStack) -> Result<IRPackage, String> {
    let tokens = lexer::tokenize(code);
    let tokens = lexer::reject_comment(&tokens);
    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = match build_ast(gathered) {
        Ok(ast) => ast,
        Err(err_token) => {
            return Err(err_token.format(&tokens, code.to_string()).to_string());
        }
    };

    let analyse_result = analyze_ast(&ast, None, dir_stack);
    for error in &analyse_result.errors {
        println!("{}", error.format(code.to_string()).bright_red());
    }
    if !analyse_result.errors.is_empty() {
        return Err("AST analysis failed".to_string());
    }
    for warn in &analyse_result.warnings {
        println!("{}", warn.format(code.to_string()).bright_yellow());
    }

    let namespace = ir_generator::NameSpace::new("Main".to_string(), None);
    let mut functions = Functions::new();
    let mut ir_generator = ir_generator::IRGenerator::new(&mut functions, namespace);

    let ir = match ir_generator.generate(&ast) {
        Ok(ir) => ir,
        Err(err) => {
            return Err(format!("Error: {:?}", err));
        }
    };

    let mut ir = ir;
    ir.push((DebugInfo { code_position: 0 }, IR::Return));
    functions.append("__main__".to_string(), ir);

    Ok(functions.build_instructions(Some(code.to_string())))
}

// Compile IR to bytecode
pub fn compile_to_bytecode(package: &IRPackage) -> Result<VMInstructionPackage, String> {
    let mut translator = IRTranslator::new(package);
    match translator.translate() {
        Ok(_) => Ok(translator.get_result()),
        Err(e) => Err(format!("IR translation failed: {:?}", e)),
    }
}