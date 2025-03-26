mod parser;
pub mod vm;
use colored::Colorize;
use rustyline::highlight::CmdKind;
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
use std::path::PathBuf;

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
    let gathered = ast_token_stream::from_stream(&tokens);
    let ast = match build_ast(&gathered) {
        Ok(ast) => ast,
        Err(err_token) => {
            return Err(format!("{}", err_token.format(&tokens, code.to_string())));
        }
    };

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
fn execute_ir(package: IRPackage, source_code: Option<String>) -> Result<(), VMError> {
    let IRPackage {
        instructions,
        function_ips,
    } = package;

    let mut coroutine_pool = VMCoroutinePool::new(true);
    let mut gc_system = GCSystem::new(None);

    let mut default_args_tuple = gc_system.new_object(VMTuple::new(vec![]));
    let mut lambda_instructions = gc_system.new_object(VMInstructions::new(instructions, function_ips));
    let mut lambda_result = gc_system.new_object(VMNull::new());
    let mut main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        &mut default_args_tuple,
        None,
        &mut lambda_instructions,
        &mut lambda_result,
    ));
    default_args_tuple.drop_ref();
    lambda_instructions.drop_ref();
    lambda_result.drop_ref();

    let mut wrapped = gc_system.new_object(VMVariableWrapper::new(&mut main_lambda));
    wrapped.clone_ref();
    main_lambda.drop_ref();

    let _coro_id =
        coroutine_pool.new_coroutine(&mut wrapped, source_code, &mut gc_system)?;

    let result = coroutine_pool.run_until_finished(&mut gc_system);
    if let Err(e) = result {
        eprintln!("{} {}", "VM Crashed!:".bright_red().underline().bold(), e.to_string());
        return Err(VMError::AssertFailed);
    }

    let result = wrapped.as_type::<VMVariableWrapper>().value_ref.as_type::<VMLambda>().get_value();

    let result_ref =
        try_value_ref_as_vmobject(result).map_err(|e| VMError::VMVariableError(e))?;

    if !result_ref.isinstance::<VMNull>() {
        match try_repr_vmobject(result_ref, None) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(err) => {
                println!("Unable to repr: {}", err.to_string());
            }
        }
    }
    gc_system.collect();

    Ok(())
}

fn execute_ir_repl(
    package: IRPackage,
    source_code: Option<String>,
    gc_system: &mut GCSystem,
    input_arguments: &mut GCRef,
) -> Result<GCRef, VMError> {
    let IRPackage {
        instructions,
        function_ips,
    } = package;

    let mut coroutine_pool = VMCoroutinePool::new(false);

    let mut key = gc_system.new_object(VMString::new("Out".to_string()));
    let mut named: GCRef = gc_system.new_object(VMNamed::new(&mut key, input_arguments));
    let mut default_args_tuple = gc_system.new_object(VMTuple::new(vec![&mut named]));
    let mut lambda_instructions = gc_system.new_object(VMInstructions::new(instructions, function_ips));
    let mut lambda_result: GCRef = gc_system.new_object(VMNull::new());
    let mut main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        &mut default_args_tuple,
        None,
        &mut lambda_instructions,
        &mut lambda_result,
    ));

    default_args_tuple.drop_ref();
    lambda_instructions.drop_ref();
    lambda_result.drop_ref();
    key.drop_ref();
    named.drop_ref();

    let mut wrapped = gc_system.new_object(VMVariableWrapper::new(&mut main_lambda));

    wrapped.clone_ref();

    main_lambda.drop_ref();
    let _coro_id =
        coroutine_pool.new_coroutine(&mut wrapped, source_code, gc_system)?;

    coroutine_pool.run_until_finished(gc_system)?;
    gc_system.collect();

    Ok(wrapped)
}

fn run_file(path: &PathBuf) -> Result<(), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "xir" {
        // Execute IR file
        match IRPackage::read_from_file(path.to_str().unwrap()) {
            Ok(package) => match execute_ir(package, None) {
                Ok(_) => {
                    Ok(())
                }
                Err(e) => Err(format!("Execution error: {}", e.to_string()).bright_red().to_string()),
            },
            Err(e) => Err(format!("Error reading IR file: {}", e).bright_red().to_string()),
        }
    } else {
        // Assume it's a source file, compile and execute
        match fs::read_to_string(path) {
            Ok(code) => match build_code(&code) {
                Ok(package) => match execute_ir(package, Some(code)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Execution error: {}", e.to_string()).bright_red().to_string()),
                },
                Err(e) => Err(format!("Compilation error: {}", e).bright_red().to_string()),
            },
            Err(e) => Err(format!("File reading error: {}", e).bright_red().to_string()),
        }
    }
}

fn compile_file(input: &PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    // Read source code
    let code = match fs::read_to_string(input) {
        Ok(content) => content,
        Err(e) => return Err(format!("Error reading source file: {}", e).bright_red().to_string()),
    };

    // Compile source code
    let package = match build_code(&code) {
        Ok(p) => p,
        Err(e) => return Err(format!("Compilation error: {}", e).bright_red().to_string()),
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
                return Err(format!("Error creating output directory: {}", e).bright_red().to_string());
            }
        }
    }

    // Write compiled IR to file
    match package.write_to_file(output_path.to_str().unwrap()) {
        Ok(_) => {
            println!(
                "Successfully saved compiled IR to: {}",
                output_path.display()
            );
            Ok(())
        }
        Err(e) => Err(format!("Error saving IR to file: {}", e).bright_red().to_string()),
    }
}

fn run_repl() -> Result<(), String> {
    use colored::*;
    use dirs::home_dir;
    use rustyline::completion::{Completer, FilenameCompleter, Pair};
    use rustyline::error::ReadlineError;
    use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
    use rustyline::hint::{Hinter, HistoryHinter};
    use rustyline::validate::{self, Validator};
    use rustyline::{CompletionType, Config, Context, EditMode, Editor};
    use std::borrow::Cow::{self, Borrowed, Owned};
    use std::collections::HashSet;
    use std::path::PathBuf;

    println!("{}", "XLang Interactive Shell".bright_blue().bold());
    println!("{}", "Type 'exit' or 'quit' to exit".bright_blue());
    println!("{}", "Use up/down arrows to navigate history".bright_blue());
    println!("{}", "Press Tab for completion suggestions".bright_blue());

    // 定义XLang关键字和自动补全器
    #[derive(rustyline_derive::Helper)]
    struct XLangHelper {
        // 可以提供文件名补全来导入文件
        file_completer: FilenameCompleter,
        // 提供历史命令提示
        history_hinter: HistoryHinter,
        // 高亮匹配的括号
        highlighter: MatchingBracketHighlighter,
        // XLang 关键字集合
        keywords: HashSet<String>,
        // 常用函数集合
        functions: HashSet<String>,
        // 内置对象集合
        objects: HashSet<String>,
        // 已定义变量
        variables: HashSet<String>,
    }

    impl XLangHelper {
        fn new() -> Self {
            let mut keywords = HashSet::new();
            let mut functions = HashSet::new();
            let mut objects = HashSet::new();

            // 添加语言关键字
            for kw in [
                "if", "else", "while", "in", "return", "break", "continue", "null", "true",
                "false", "and", "or", "not", "bind", "self", "async", "await", "yield", "wrap",
                "selfof", "import", "typeof", "copy", "deepcopy", "wipe", "aliasof", "keyof",
                "valueof",
            ] {
                keywords.insert(kw.to_string());
            }

            // 添加常用函数
            for func in ["print", "input", "len", "str", "int", "float", "bool"] {
                functions.insert(func.to_string());
            }

            // 添加内置对象
            for obj in ["Out"] {
                objects.insert(obj.to_string());
            }

            XLangHelper {
                file_completer: FilenameCompleter::new(),
                history_hinter: HistoryHinter {},
                highlighter: MatchingBracketHighlighter::new(),
                keywords,
                functions,
                objects,
                variables: HashSet::new(),
            }
        }

        // 从代码中提取定义的变量
        fn extract_variables(&mut self, code: &str) {
            // 简单解析，查找 := 定义的变量
            for line in code.lines() {
                let line = line.trim();
                if let Some(idx) = line.find(":=") {
                    if let Some(var_name) = line[..idx].trim().split_whitespace().last() {
                        self.variables.insert(var_name.to_string());
                    }
                }
            }
        }
    }

    impl Completer for XLangHelper {
        type Candidate = Pair;

        fn complete(
            &self,
            line: &str,
            pos: usize,
            ctx: &Context<'_>,
        ) -> rustyline::Result<(usize, Vec<Pair>)> {
            // 获取光标前的单词用于补全
            let (start, word) = {
                let mut cs = line[..pos].char_indices().rev();
                let mut word_chars = Vec::new();
                let mut word_start = pos;

                while let Some((idx, c)) = cs.next() {
                    if c.is_alphanumeric() || c == '_' || c == '.' {
                        word_chars.push(c);
                        word_start = idx;
                    } else {
                        break;
                    }
                }

                word_chars.reverse();
                let word: String = word_chars.into_iter().collect();
                (word_start, word)
            };

            if word.contains('/') || word.contains('\\') {
                // 如果是文件路径，使用文件补全
                return self.file_completer.complete(line, pos, ctx);
            }

            let mut completions = Vec::new();
            // 关键字补全
            for keyword in &self.keywords {
                if keyword.starts_with(&word) {
                    completions.push(Pair {
                        display: keyword.clone(),
                        replacement: keyword.clone(),
                    });
                }
            }

            // 函数补全
            for func in &self.functions {
                if func.starts_with(&word) {
                    completions.push(Pair {
                        display: format!("{}()", func),
                        replacement: format!("{}(", func),
                    });
                }
            }

            // 对象补全
            for obj in &self.objects {
                if obj.starts_with(&word) {
                    completions.push(Pair {
                        display: obj.clone(),
                        replacement: obj.clone(),
                    });
                }
            }

            // 变量补全
            for var in &self.variables {
                if var.starts_with(&word) {
                    completions.push(Pair {
                        display: var.clone(),
                        replacement: var.clone(),
                    });
                }
            }

            // 对补全选项排序
            completions.sort_by(|a, b| a.display.cmp(&b.display));

            Ok((start, completions))
        }
    }

    impl Highlighter for XLangHelper {
        fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
            self.highlighter.highlight(line, pos)
        }

        fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
            &'s self,
            prompt: &'p str,
            default: bool,
        ) -> Cow<'b, str> {
            if default {
                Borrowed(prompt)
            } else {
                Owned(prompt.bright_purple().to_string())
            }
        }

        fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
            Owned(hint.bright_cyan().italic().underline().to_string())
        }

        fn highlight_char(&self, line: &str, pos: usize, forced: CmdKind) -> bool {
            self.highlighter.highlight_char(line, pos, forced)
        }
    }

    impl Hinter for XLangHelper {
        type Hint = String;

        fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
            self.history_hinter.hint(line, pos, ctx)
        }
    }

    impl Validator for XLangHelper {
        fn validate(
            &self,
            _ctx: &mut validate::ValidationContext,
        ) -> rustyline::Result<validate::ValidationResult> {
            Ok(validate::ValidationResult::Valid(None))
        }
    }

    // 设置Rustyline配置
    let _config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    // 创建编辑器实例并设置补全器
    let mut rl = Editor::new().map_err(|e| format!("Editor creation error: {}", e))?;
    rl.set_helper(Some(XLangHelper::new()));

    // 设置历史文件路径
    let history_path = home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xlang_history");

    // 尝试加载历史
    if history_path.exists() {
        if let Err(e) = rl.load_history(&history_path) {
            eprintln!(
                "{}",
                format!("Warning: Could not load history: {}", e).yellow()
            );
        }
    }

    let mut gc_system = GCSystem::new(None);
    let mut input_arguments = gc_system.new_object(VMTuple::new(vec![]));
    //let _wrapper = gc_system.new_object(VMVariableWrapper::new(input_arguments.clone()));
    //input_arguments.offline();

    let mut line_count = 0;

    loop {
        // 开始一个新的多行输入
        let mut input_buffer = String::new();
        let mut is_multiline = false;

        // 主提示符
        let main_prompt = format!("{} ", format!("In[{}]:", line_count).green().bold());

        // 读取第一行
        let readline = rl.readline(&main_prompt);

        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                // 处理退出命令
                if !is_multiline && (input == "exit" || input == "quit") {
                    println!("{}", "Exit".yellow());
                    break;
                }

                // 添加到输入缓冲区
                input_buffer.push_str(&line);
                input_buffer.push('\n');

                // 检查输入是否完整
                if !is_input_complete(&input_buffer) {
                    is_multiline = true;

                    // 继续读取行直到输入完整
                    while is_multiline {
                        // 继续提示符
                        let cont_prompt = format!("{} ", "...".bright_yellow());
                        match rl.readline(&cont_prompt) {
                            Ok(cont_line) => {
                                // 添加到输入缓冲区
                                input_buffer.push_str(&cont_line);
                                input_buffer.push('\n');

                                // 检查输入是否现在完整
                                if is_input_complete(&input_buffer) {
                                    is_multiline = false;
                                }
                            }
                            Err(ReadlineError::Interrupted) => {
                                println!("{}", "Multiline input cancelled".yellow());
                                input_buffer.clear();
                                is_multiline = false;
                                continue;
                            }
                            Err(err) => {
                                println!("{}", format!("Input error: {}", err).red());
                                is_multiline = false;
                                continue;
                            }
                        }
                    }
                }

                // 从输入中提取变量
                if let Some(helper) = rl.helper_mut() {
                    helper.extract_variables(&input_buffer);
                }

                // 将完整输入添加到历史记录 - 修复多余换行问题
                let history_entry = input_buffer.trim_end().to_string();
                if !history_entry.is_empty() {
                    let _ = rl.add_history_entry(history_entry);
                }

                // 处理完整输入
                match build_code(&input_buffer) {
                    Ok(package) => match execute_ir_repl(
                        package,
                        Some(input_buffer.to_string()),
                        &mut gc_system,
                        &mut input_arguments,
                    ) {
                        Ok(mut lambda_ref) => {
                            let executed = lambda_ref.as_const_type::<VMVariableWrapper>().value_ref.as_const_type::<VMLambda>();
                            let result_ref = executed.result.clone();
                            let idx = input_arguments.as_type::<VMTuple>().values.len();
                            match try_repr_vmobject(result_ref.clone(), None) {
                                Ok(value) => {
                                    if !result_ref.isinstance::<VMNull>() {
                                        println!(
                                            "{} = {}",
                                            format!("Out[{}]", idx).blue().bold(),
                                            value.bright_white().bold()
                                        );
                                    }
                                    let mut result_ref = result_ref.clone();
                                    let result = input_arguments
                                        .as_type::<VMTuple>()
                                        .append(&mut result_ref);
                                    if result.is_err() {
                                        println!(
                                            "{}",
                                            format!("Error: {}", result.unwrap_err().to_string()).red()
                                        );
                                    }
                                }
                                Err(err) => {
                                    println!(
                                        "{}{}",
                                        format!("Out[{}]", idx).blue().bold(),
                                        format!("<Unable to repr: {}>", err.to_string()).red()
                                    );
                                }
                            }
                            lambda_ref.drop_ref();
                        }
                        Err(e) => println!(
                            "{}",
                            format!("Execution error: {}", e.to_string()).red().bold().underline()
                        ),
                    },
                    Err(e) => println!("{}", format!("Compilation error: {}", e).red().bold()),
                }
                gc_system.collect();
                line_count = input_arguments.as_type::<VMTuple>().values.len();
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Interrupted".yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Exit".yellow());
                input_arguments.drop_ref();
                break;
            }
            Err(err) => {
                println!("{}", format!("Input error: {}", err).red());
                break;
            }
        }
    }

    // 保存历史
    if let Err(e) = rl.save_history(&history_path) {
        eprintln!(
            "{}",
            format!("Warning: Could not save history: {}", e).yellow()
        );
    }

    Ok(())
}
// Function to check if input is complete
fn is_input_complete(input: &str) -> bool {
    // Simple bracket matching
    let mut brace_count = 0; // { }
    let mut bracket_count = 0; // [ ]
    let mut paren_count = 0; // ( )

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
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Run { input } => {
            if let Err(e) = run_file(&input) {
                eprintln!("{}", e);
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
