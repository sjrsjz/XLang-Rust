mod parser;
pub mod vm;
use vm::executor::variable::VMInstructions;
use vm::executor::variable::VMLambda;
use vm::executor::variable::VMTuple;
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 编译源代码到 XLang IR 文件
    Compile {
        /// 输入源代码文件路径
        #[arg(required = true)]
        input: PathBuf,

        /// 输出 IR 文件路径 (默认为与输入同名但扩展名为 .xir)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// 执行 XLang 源代码或编译后的 IR 文件
    Run {
        /// 输入文件路径 (源代码或 .xir 文件)
        #[arg(required = true)]
        input: PathBuf,
    },

    /// 交互式解释器模式
    Repl {},
}

// 编译代码，生成中间表示
fn build_code(code: &str) -> Result<IRPackage, String> {
    let tokens = lexer::reject_comment(lexer::tokenize(code));
    // 可选：打印tokens
    // for token in &tokens {
    //     print!("{:?} ", token.to_string());
    // }

    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = match build_ast(&gathered) {
        Ok(ast) => ast,
        Err(err_token) => {
            return Err(format!("Error token: {:?}", err_token.format(&tokens)));
        }
    };

    // 可选：打印AST
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

// 执行编译后的代码
fn execute_ir(package: IRPackage, source_code: Option<String>) -> Result<GCRef, VMError> {
    let IRPackage {
        instructions,
        function_ips,
    } = package;

    let mut coroutine_pool = VMCoroutinePool::new();
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
        println!("执行错误: {}", e.to_string());
        return Err(VMError::AssertFailed);
    }

    let result = main_lambda.as_const_type::<VMLambda>().result.clone();

    let result_ref =
        try_value_ref_as_vmobject(result.clone()).map_err(|e| VMError::VMVariableError(e))?;

    if !result_ref.isinstance::<VMNull>() {
        match try_repr_vmobject(result_ref.clone()) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(err) => {
                println!("Unable to repr: {}", err.to_string());
            }
        }
    }
    main_lambda.offline();

    println!("done!");
    gc_system.collect();

    Ok(result)
}

fn run_file(path: &PathBuf) -> Result<(), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "xir" {
        // 执行 IR 文件
        match IRPackage::read_from_file(path.to_str().unwrap()) {
            Ok(package) => match execute_ir(package, None) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("执行错误: {}", e.to_string())),
            },
            Err(e) => Err(format!("读取 IR 文件错误: {}", e)),
        }
    } else {
        // 假设是源码文件，先编译再执行
        match fs::read_to_string(path) {
            Ok(code) => match build_code(&code) {
                Ok(package) => match execute_ir(package, Some(code)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("执行错误: {}", e.to_string())),
                },
                Err(e) => Err(format!("编译错误: {}", e)),
            },
            Err(e) => Err(format!("读取文件错误: {}", e)),
        }
    }
}

fn compile_file(input: &PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    // 读取源代码
    let code = match fs::read_to_string(input) {
        Ok(content) => content,
        Err(e) => return Err(format!("读取源文件错误: {}", e)),
    };

    // 编译源代码
    let package = match build_code(&code) {
        Ok(p) => p,
        Err(e) => return Err(format!("编译错误: {}", e)),
    };

    // 确定输出路径
    let output_path = match output {
        Some(path) => path,
        None => {
            let mut path = input.clone();
            path.set_extension("xir");
            path
        }
    };

    // 创建目录（如果需要）
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("创建输出目录错误: {}", e));
            }
        }
    }

    // 写入编译后的 IR 到文件
    match package.write_to_file(output_path.to_str().unwrap()) {
        Ok(_) => {
            println!("成功将编译后的 IR 保存到: {}", output_path.display());
            Ok(())
        }
        Err(e) => Err(format!("保存 IR 到文件错误: {}", e)),
    }
}

fn run_repl() -> Result<(), String> {
    println!("XLang 交互式解释器");
    println!("输入 'exit' 或 'quit' 退出");

    let mut gc_system = GCSystem::new(None);

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            println!("读取输入错误");
            continue;
        }

        let input = input.trim();
        if input == "exit" || input == "quit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        match build_code(input) {
            Ok(package) => match execute_ir(package, Some(input.to_string())) {
                Ok(_) => {}
                Err(e) => println!("执行错误: {}", e.to_string()),
            },
            Err(e) => println!("编译错误: {}", e),
        }
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output } => {
            if let Err(e) = compile_file(&input, output) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Run { input } => {
            if let Err(e) = run_file(&input) {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Repl {} => {
            if let Err(e) = run_repl() {
                eprintln!("REPL 错误: {}", e);
                std::process::exit(1);
            }
        }
    }
}
