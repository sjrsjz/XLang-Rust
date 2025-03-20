mod parser;
pub mod vm;
use vm::executor::variable::VMInstructions;
use vm::executor::variable::VMLambda;
use vm::executor::variable::VMTuple;
use vm::gc;
use vm::gc::gc::GCRef;
use vm::ir::IRPackage;
use vm::ir::IR;

use self::parser::ast::ast_token_stream;
use self::parser::ast::build_ast;
use self::parser::lexer::{lexer, Token, TokenType};

use self::vm::gc::gc::GCSystem;

use self::vm::executor::variable::*;
use self::vm::executor::vm::*;
use self::vm::ir::Functions;
use self::vm::ir_generator::ir_generator;

use clap::{Parser, Subcommand};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::result;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile source code to XLang IR file
    Compile {
        /// Input source code file path
        #[arg(required = true)]
        input: PathBuf,

        /// Output IR file path (defaults to same name as input but with .xir extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Execute XLang source code or compiled IR file
    Run {
        /// Input file path (source code or .xir file)
        #[arg(required = true)]
        input: PathBuf,
    },

    /// Interactive interpreter mode
    Repl {},
}

// Compile code and generate intermediate representation
fn build_code(code: &str) -> Result<IRPackage, String> {
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    // Optional: Print tokens
    // for token in &tokens {
    //     print!("{:?} ", token.to_string());
    // }

    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = match build_ast(&gathered) {
        Ok(ast) => ast,
        Err(err_token) => {
            return Err(format!("{}", err_token.format(&tokens, code.to_string())));
        }
    };

    // Optional: Print AST
    // println!("\n\nAST:\n");
    // ast.formatted_print(0);

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
    ir.push(IR::Return);
    functions.append("__main__".to_string(), ir);

    return Ok(functions.build_instructions());
}

// Execute compiled code
fn execute_ir(package: IRPackage, source_code: Option<String>) -> Result<GCRef, VMError> {
    let IRPackage {
        instructions,
        function_ips,
    } = package;

    let mut coroutine_pool = VMCoroutinePool::new(true);
    let mut gc_system = GCSystem::new(None);

    let default_args_tuple = gc_system.new_object(VMTuple::new(vec![]));
    let lambda_instructions = gc_system.new_object(VMInstructions::new(instructions, function_ips));

    let lambda_result = gc_system.new_object(VMNull::new());
    let main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        default_args_tuple.clone(),
        None,
        lambda_instructions.clone(),
        lambda_result.clone(),
    ));
    default_args_tuple.offline();
    lambda_instructions.offline();
    lambda_result.offline();

    let _coro_id =
        coroutine_pool.new_coroutine(main_lambda.clone(), source_code, &mut gc_system, true)?;

    let result = coroutine_pool.run_until_finished(&mut gc_system);
    if let Err(e) = result {
        println!("VM Crashed!: {}", e.to_string());
        return Err(VMError::AssertFailed);
    }

    let result = main_lambda.as_const_type::<VMLambda>().result.clone();

    let result_ref =
        try_value_ref_as_vmobject(result.clone()).map_err(|e| VMError::VMVariableError(e))?;

    if !result_ref.isinstance::<VMNull>() {
        match try_repr_vmobject(result_ref.clone(), None) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(err) => {
                println!("Unable to repr: {}", err.to_string());
            }
        }
    }
    main_lambda.offline();

    gc_system.collect();

    Ok(result)
}


fn execute_ir_repl(package: IRPackage, source_code: Option<String>, gc_system:&mut GCSystem, input_arguments: GCRef) -> Result<GCRef, VMError> {
    let IRPackage {
        instructions,
        function_ips,
    } = package;

    let mut coroutine_pool = VMCoroutinePool::new(false);

    let key = gc_system.new_object(VMString::new("Out".to_string()));
    let named = gc_system.new_object(VMNamed::new(key.clone(), input_arguments.clone()));
    let default_args_tuple = gc_system.new_object(VMTuple::new(vec![named.clone()]));
    key.offline();
    named.offline();
    let lambda_instructions = gc_system.new_object(VMInstructions::new(instructions, function_ips));

    let lambda_result: GCRef = gc_system.new_object(VMNull::new());
    let main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        default_args_tuple.clone(),
        None,
        lambda_instructions.clone(),
        lambda_result.clone(),
    ));
    default_args_tuple.offline();
    lambda_instructions.offline();
    lambda_result.offline();

    let _coro_id =
        coroutine_pool.new_coroutine(main_lambda.clone(), source_code, gc_system, true)?;
    coroutine_pool.run_until_finished(gc_system)?;
    gc_system.collect();

    Ok(main_lambda)
}



fn run_file(path: &PathBuf) -> Result<(), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "xir" {
        // Execute IR file
        match IRPackage::read_from_file(path.to_str().unwrap()) {
            Ok(package) => match execute_ir(package, None) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Execution error: {}", e.to_string())),
            },
            Err(e) => Err(format!("Error reading IR file: {}", e)),
        }
    } else {
        // Assume it's a source file, compile and execute
        match fs::read_to_string(path) {
            Ok(code) => match build_code(&code) {
                Ok(package) => match execute_ir(package, Some(code)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Execution error: {}", e.to_string())),
                },
                Err(e) => Err(format!("Compilation error: {}", e)),
            },
            Err(e) => Err(format!("File reading error: {}", e)),
        }
    }
}

fn compile_file(input: &PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    // Read source code
    let code = match fs::read_to_string(input) {
        Ok(content) => content,
        Err(e) => return Err(format!("Error reading source file: {}", e)),
    };

    // Compile source code
    let package = match build_code(&code) {
        Ok(p) => p,
        Err(e) => return Err(format!("Compilation error: {}", e)),
    };

    // Determine output path
    let output_path = match output {
        Some(path) => path,
        None => {
            let mut path = input.clone();
            path.set_extension("xir");
            path
        }
    };

    // Create directories if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Error creating output directory: {}", e));
            }
        }
    }

    // Write compiled IR to file
    match package.write_to_file(output_path.to_str().unwrap()) {
        Ok(_) => {
            println!("Successfully saved compiled IR to: {}", output_path.display());
            Ok(())
        }
        Err(e) => Err(format!("Error saving IR to file: {}", e)),
    }
}

fn run_repl() -> Result<(), String> {
    use rustyline::Editor;
    use rustyline::error::ReadlineError;
    use colored::*;
    use dirs::home_dir;
    use std::path::PathBuf;

    println!("{}", "XLang Interactive Shell".bright_blue().bold());
    println!("{}", "Type 'exit' or 'quit' to exit".bright_blue());
    println!("{}", "Use up/down arrows to navigate history".bright_blue());

    // Set history file path
    let history_path = home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xlang_history");

    // Create editor instance
    let mut rl = Editor::<(), _>::new().map_err(|e| format!("Editor creation error: {}", e))?;
    
    // Try to load history
    if history_path.exists() {
        if let Err(e) = rl.load_history(&history_path) {
            eprintln!("{}", format!("Warning: Could not load history: {}", e).yellow());
        }
    }

    let mut gc_system = GCSystem::new(None);
    let input_arguments = gc_system.new_object(VMTuple::new(vec![]));
    let _wrapper = gc_system.new_object(VMVariableWrapper::new(input_arguments.clone()));
    input_arguments.offline();

    let mut line_count = 0;

    loop {
        // Start with an empty buffer for collecting multi-line input
        let mut input_buffer = String::new();
        let mut is_multiline = false;
        
        // Main prompt for the first line
        let main_prompt = format!("{} ", format!("In[{}]:", line_count).green().bold());
        
        // Read the first line
        let readline = rl.readline(&main_prompt);
        
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                
                // Process exit commands on the first line
                if !is_multiline && (input == "exit" || input == "quit") {
                    println!("{}", "Goodbye!".bright_blue());
                    break;
                }
                
                // Add to input buffer
                input_buffer.push_str(&line);
                input_buffer.push('\n');
                
                // Check if input is complete
                if !is_input_complete(&input_buffer) {
                    is_multiline = true;
                    
                    // Continue reading lines until input is complete
                    while is_multiline {
                        // Continuation prompt
                        let cont_prompt = format!("{} ", "...".bright_yellow());
                        match rl.readline(&cont_prompt) {
                            Ok(cont_line) => {
                                // Add to input buffer
                                input_buffer.push_str(&cont_line);
                                input_buffer.push('\n');
                                
                                // Check if input is now complete
                                if is_input_complete(&input_buffer) {
                                    is_multiline = false;
                                }
                            },
                            Err(ReadlineError::Interrupted) => {
                                println!("{}", "Multiline input cancelled".yellow());
                                input_buffer.clear();
                                is_multiline = false;
                                continue;
                            },
                            Err(err) => {
                                println!("{}", format!("Input error: {}", err).red());
                                is_multiline = false;
                                continue;
                            }
                        }
                    }
                }
                
                // Add the complete input to history
                let _ = rl.add_history_entry(input_buffer.clone());
                
                // Process the complete input
                match build_code(&input_buffer) {
                    Ok(package) => match execute_ir_repl(package, Some(input_buffer.to_string()), &mut gc_system, input_arguments.clone()) {
                        Ok(lambda_ref) => {
                            let executed = lambda_ref.as_const_type::<VMLambda>();
                            let result_ref = executed.result.clone();
                            let idx = input_arguments.as_type::<VMTuple>().values.len();
                            match try_repr_vmobject(result_ref.clone(), None) {
                                Ok(value) => {
                                    if !result_ref.isinstance::<VMNull>() {
                                        println!("{} = {}", format!("Out[{}]", idx).blue().bold(), value.bright_white().bold());
                                    }
                                    let mut result_ref = result_ref.clone();
                                    input_arguments.get_traceable().add_reference(&mut result_ref);
                                    input_arguments.as_type::<VMTuple>().values.push(result_ref.clone());
                                    result_ref.offline();
                                }
                                Err(err) => {
                                    println!("{}{}", 
                                             format!("Out[{}]", idx).blue().bold(), 
                                             format!("<Unable to repr: {}>", err.to_string()).red());
                                }
                            }
                            lambda_ref.offline();
                        }
                        Err(e) => println!("{}", format!("Execution error: {}", e.to_string()).red().bold()),
                    },
                    Err(e) => println!("{}", format!("Compilation error: {}", e).red().bold()),
                }
                
                line_count = input_arguments.as_type::<VMTuple>().values.len();
            },
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Interrupted".yellow());
                continue;
            },
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".bright_blue());
                break;
            },
            Err(err) => {
                println!("{}", format!("Input error: {}", err).red());
                break;
            }
        }
    }

    // Save history
    if let Err(e) = rl.save_history(&history_path) {
        eprintln!("{}", format!("Warning: Could not save history: {}", e).yellow());
    }

    Ok(())
}

// Function to check if input is complete
fn is_input_complete(input: &str) -> bool {
    // Simple bracket matching
    let mut brace_count = 0;     // { }
    let mut bracket_count = 0;   // [ ]
    let mut paren_count = 0;     // ( )
    
    // Track string literals to ignore brackets inside them
    let mut in_string = false;
    let mut escape_next = false;
    
    for c in input.chars() {
        if in_string {
            if escape_next {
                escape_next = false;
            } else if c == '\\' {
                escape_next = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' => in_string = true,
            '{' => brace_count += 1,
            '}' => brace_count -= 1,
            '[' => bracket_count += 1,
            ']' => bracket_count -= 1,
            '(' => paren_count += 1,
            ')' => paren_count -= 1,
            _ => {}
        }
    }
    
    // Also try to tokenize the input using the lexer
    let tokens = lexer::reject_comment(lexer::tokenize(input));
    
    // Input is incomplete if:
    // 1. Any bracket count is unbalanced
    // 2. We're still in a string
    // 3. The input ends with an operator or other continuation indicator
    let balanced = brace_count == 0 && bracket_count == 0 && paren_count == 0 && !in_string;
    
    // Check for trailing operators that indicate continuation
    let last_token = tokens.last().map(|t| t.token).unwrap_or("");
    let is_trailing_operator = ["+", "-", "*", "/", "=", ":=", ".", "->"].contains(&last_token);
    
    // Check for trailing semicolon
    let trimmed = input.trim();
    let ends_with_semicolon = trimmed.ends_with(';');
    
    balanced && !is_trailing_operator && (ends_with_semicolon || !trimmed.contains(';'))
}
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output } => {
            if let Err(e) = compile_file(&input, output) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Run { input } => {
            if let Err(e) = run_file(&input) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Repl {} => {
            if let Err(e) = run_repl() {
                eprintln!("REPL error: {}", e);
                std::process::exit(1);
            }
        }
    }
}