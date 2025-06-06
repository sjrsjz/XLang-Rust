mod lsp;
use colored::Colorize;
use rustyline::highlight::CmdKind;

mod stdlib;
use crate::stdlib::inject_builtin_functions;
use xlang_vm_core::executor::variable::VMInstructions;
use xlang_vm_core::executor::variable::VMLambda;
use xlang_vm_core::executor::variable::VMTuple;
use xlang_vm_core::executor::variable::*;
use xlang_vm_core::executor::vm::*;
use xlang_vm_core::gc::GCRef;
use xlang_vm_core::gc::GCSystem;
use xlang_vm_core::instruction_set::VMInstructionPackage;
use xlang_vm_core::ir::IRPackage;
use xlang_vm_core::ir_translator::IRTranslator;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use xlang_frontend::compile::{build_code, compile_to_bytecode};
use xlang_frontend::dir_stack::DirStack;
use xlang_frontend::parser::lexer::lexer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile source code to XLang IR file or bytecode
    Compile {
        /// Input source code file path
        #[arg(required = true)]
        input: PathBuf,

        /// Output file path (defaults to same name as input but with .xir or .xbc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compile directly to bytecode instead of IR
        #[arg(short, long)]
        bytecode: bool,
    },

    /// Execute XLang source code, IR file, or bytecode file
    Run {
        /// Input file path (source code, .xir file, or .xbc bytecode file)
        #[arg(required = true)]
        input: PathBuf,
    },

    /// Display IR file content
    DisplayIR {
        /// Input IR file path
        #[arg(required = true)]
        input: PathBuf,
    },

    /// Translate IR file to bytecode file
    Translate {
        /// Input IR file path
        #[arg(required = true)]
        input: PathBuf,

        /// Output bytecode file path (defaults to same name as input but with .xbc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Interactive interpreter mode
    Repl {},

    /// LSP server mode
    Lsp {
        /// LSP server port
        #[arg(short, long, default_value_t = 2087)]
        port: u16,
    },
}

// Execute compiled code
fn execute_ir(package: VMInstructionPackage, _dir_stack: &mut DirStack) -> Result<(), VMError> {
    let mut coroutine_pool = VMCoroutinePool::new(true);
    let mut gc_system = GCSystem::new(None);

    let mut default_args_tuple = gc_system.new_object(VMTuple::new(&mut vec![]));
    let mut lambda_instructions = gc_system.new_object(VMInstructions::new(&package));
    let mut lambda_result = gc_system.new_object(VMNull::new());
    let mut main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        &mut default_args_tuple,
        None,
        None,
        &mut VMLambdaBody::VMInstruction(lambda_instructions.clone()),
        &mut lambda_result,
        false,
    ));
    lambda_instructions.drop_ref();
    lambda_result.drop_ref();

    main_lambda.clone_ref();

    let coro_id =
        coroutine_pool.new_coroutine(&mut main_lambda, &mut default_args_tuple, &mut gc_system)?;

    let result = inject_builtin_functions(
        coroutine_pool
            .get_executor_mut(coro_id)
            .unwrap()
            .get_context_mut(),
        &mut gc_system,
    );

    if let Err(mut e) = result {
        eprintln!(
            "{} {}",
            "VM Crashed!:".bright_red().underline().bold(),
            e.to_string()
        );
        e.consume_ref();
        return Err(VMError::AssertFailed);
    }

    let result = coroutine_pool.run_until_finished(&mut gc_system);
    if let Err(mut e) = result {
        eprintln!(
            "{} {}",
            "VM Crashed!:".bright_red().underline().bold(),
            e.to_string()
        );
        e.consume_ref();
        main_lambda.drop_ref();
        gc_system.collect();
        return Err(VMError::AssertFailed);
    }

    let result = main_lambda.as_type::<VMLambda>().get_value();

    if !result.isinstance::<VMNull>() {
        match try_repr_vmobject(result, None) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(mut err) => {
                println!("Unable to repr: {}", err.to_string());
                err.consume_ref();
            }
        }
    }
    main_lambda.drop_ref();
    gc_system.collect();
    Ok(())
}

fn execute_ir_repl(
    package: VMInstructionPackage,
    gc_system: &mut GCSystem,
    input_arguments: &mut GCRef,
) -> Result<GCRef, VMError> {
    let mut coroutine_pool = VMCoroutinePool::new(false);

    let mut key = gc_system.new_object(VMString::new("Out"));
    let mut named: GCRef = gc_system.new_object(VMNamed::new(&mut key, input_arguments));
    let mut default_args_tuple = gc_system.new_object(VMTuple::new(&mut vec![&mut named]));
    let mut lambda_instructions = gc_system.new_object(VMInstructions::new(&package));
    let mut lambda_result: GCRef = gc_system.new_object(VMNull::new());
    let mut main_lambda = gc_system.new_object(VMLambda::new(
        0,
        "__main__".to_string(),
        &mut default_args_tuple,
        None,
        None,
        &mut VMLambdaBody::VMInstruction(lambda_instructions.clone()),
        &mut lambda_result,
        false,
    ));

    lambda_instructions.drop_ref();
    lambda_result.drop_ref();
    key.drop_ref();
    named.drop_ref();

    let mut wrapped = main_lambda.clone_ref();

    wrapped.clone_ref();

    main_lambda.drop_ref();
    let coro_id = coroutine_pool.new_coroutine(&mut wrapped, &mut default_args_tuple, gc_system)?;

    let result = inject_builtin_functions(
        coroutine_pool
            .get_executor_mut(coro_id)
            .unwrap()
            .get_context_mut(),
        gc_system,
    );

    if let Err(mut e) = result {
        eprintln!(
            "{} {}",
            "VM Crashed!:".bright_red().underline().bold(),
            e.to_string()
        );
        e.consume_ref();
        wrapped.drop_ref();
        return Err(VMError::AssertFailed);
    }

    coroutine_pool.run_until_finished(gc_system)?;
    gc_system.collect();

    Ok(wrapped)
}

fn run_file(path: &PathBuf) -> Result<(), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match extension {
        "xir" => {
            // Execute IR file
            match IRPackage::read_from_file(path.to_str().unwrap()) {
                Ok(package) => {
                    let mut translator = IRTranslator::new(&package);
                    let translate_result = translator.translate();
                    let result = if translate_result.is_ok() {
                        translator.get_result()
                    } else {
                        return Err(format!(
                            "IR translation failed: {:?}",
                            translate_result.err().unwrap()
                        )
                        .bright_red()
                        .to_string());
                    };
                    let mut dir_stack = DirStack::new(Some(
                        &path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .to_path_buf(),
                    ))
                    .unwrap();

                    match execute_ir(result, &mut dir_stack) {
                        Ok(_) => Ok(()),
                        Err(mut e) => {
                            let err = Err(format!("Execution error: {}", e.to_string())
                                .bright_red()
                                .to_string());
                            e.consume_ref();
                            err
                        }
                    }
                }
                Err(e) => Err(format!("Error reading IR file: {}", e)
                    .bright_red()
                    .to_string()),
            }
        }
        "xbc" => {
            // Execute bytecode file directly
            match VMInstructionPackage::read_from_file(path.to_str().unwrap()) {
                Ok(bytecode) => {
                    let mut dir_stack = DirStack::new(Some(
                        &path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .to_path_buf(),
                    ))
                    .unwrap();
                    match execute_ir(bytecode, &mut dir_stack) {
                        Ok(_) => Ok(()),
                        Err(mut e) => {
                            let err = Err(format!("Execution error: {}", e.to_string())
                                .bright_red()
                                .to_string());
                            e.consume_ref();
                            err
                        }
                    }
                }
                Err(e) => Err(format!("Error reading bytecode file: {}", e)
                    .bright_red()
                    .to_string()),
            }
        }
        _ => {
            // Assume it's a source file, compile and execute
            match fs::read_to_string(path) {
                Ok(code) => {
                    let dir_stack = DirStack::new(Some(
                        &path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .to_path_buf(),
                    ));
                    if dir_stack.is_err() {
                        return Err(format!(
                            "Error creating directory stack: {}",
                            dir_stack.err().unwrap()
                        )
                        .bright_red()
                        .to_string());
                    }
                    let mut dir_stack = dir_stack.unwrap();
                    match build_code(&code, &mut dir_stack) {
                        Ok(package) => {
                            let mut translator = IRTranslator::new(&package);
                            let translate_result = translator.translate();
                            let result = if translate_result.is_ok() {
                                translator.get_result()
                            } else {
                                return Err(format!(
                                    "IR translation failed: {:?}",
                                    translate_result.err().unwrap()
                                )
                                .bright_red()
                                .to_string());
                            };
                            match execute_ir(result, &mut dir_stack) {
                                Ok(_) => Ok(()),
                                Err(mut e) => {
                                    let err = Err(format!("Execution error: {}", e.to_string())
                                        .bright_red()
                                        .to_string());
                                    e.consume_ref();
                                    err
                                }
                            }
                        }
                        Err(e) => Err(format!("Compilation error: {}", e).bright_red().to_string()),
                    }
                }
                Err(e) => Err(format!("File reading error: {}", e)
                    .bright_red()
                    .to_string()),
            }
        }
    }
}

fn ir_to_bytecode_file(input: &PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    // Read IR file
    let package = match IRPackage::read_from_file(input.to_str().unwrap()) {
        Ok(p) => p,
        Err(e) => {
            return Err(format!("Error reading IR file: {}", e)
                .bright_red()
                .to_string())
        }
    };

    // Translate IR to bytecode
    let bytecode = match compile_to_bytecode(&package) {
        Ok(b) => b,
        Err(e) => return Err(format!("Translation error: {}", e).bright_red().to_string()),
    };

    // Determine output path
    let output_path = match output {
        Some(path) => path,
        None => {
            let mut path = input.clone();
            path.set_extension("xbc");
            path
        }
    };

    // Create directories if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Error creating output directory: {}", e)
                    .bright_red()
                    .to_string());
            }
        }
    }

    // Write bytecode to file
    match bytecode.write_to_file(output_path.to_str().unwrap()) {
        Ok(_) => {
            println!("Successfully saved bytecode to: {}", output_path.display());
            Ok(())
        }
        Err(e) => Err(format!("Error saving bytecode to file: {}", e)
            .bright_red()
            .to_string()),
    }
}

fn compile_file(input: &PathBuf, output: Option<PathBuf>, bytecode: bool) -> Result<(), String> {
    let mut dir_stack = DirStack::new(Some(
        &input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    ))
    .unwrap();
    // Read source code
    let code = match fs::read_to_string(input) {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Error reading source file: {}", e)
                .bright_red()
                .to_string())
        }
    };

    // Compile source code to IR
    let ir_package = match build_code(&code, &mut dir_stack) {
        Ok(p) => p,
        Err(e) => return Err(format!("Compilation error: {}", e).bright_red().to_string()),
    };

    if bytecode {
        // Compile directly to bytecode
        let bytecode = match compile_to_bytecode(&ir_package) {
            Ok(b) => b,
            Err(e) => {
                return Err(format!("Bytecode generation error: {}", e)
                    .bright_red()
                    .to_string())
            }
        };

        // Determine output path
        let output_path = match output {
            Some(path) => path,
            None => {
                let mut path = input.clone();
                path.set_extension("xbc");
                path
            }
        };

        // Create directories if needed
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Err(format!("Error creating output directory: {}", e)
                        .bright_red()
                        .to_string());
                }
            }
        }

        // Write bytecode to file
        match bytecode.write_to_file(output_path.to_str().unwrap()) {
            Ok(_) => {
                println!("Successfully saved bytecode to: {}", output_path.display());
                Ok(())
            }
            Err(e) => Err(format!("Error saving bytecode to file: {}", e)
                .bright_red()
                .to_string()),
        }
    } else {
        // Just compile to IR (original functionality)
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
                    return Err(format!("Error creating output directory: {}", e)
                        .bright_red()
                        .to_string());
                }
            }
        }

        // Write compiled IR to file
        match ir_package.write_to_file(output_path.to_str().unwrap()) {
            Ok(_) => {
                println!(
                    "Successfully saved compiled IR to: {}",
                    output_path.display()
                );
                Ok(())
            }
            Err(e) => Err(format!("Error saving IR to file: {}", e)
                .bright_red()
                .to_string()),
        }
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
    use std::borrow::Cow::{self, Owned};
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
                "false", "and", "or", "not", "bind", "self", "async", "await", "emit", "wrap",
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
            {
                let obj = "Out";
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
                    if let Some(var_name) = line[..idx].split_whitespace().last() {
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
                let cs = line[..pos].char_indices().rev();
                let mut word_chars = Vec::new();
                let mut word_start = pos;

                for (idx, c) in cs {
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
                Owned(prompt.green().bold().to_string())
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
    let mut input_arguments = gc_system.new_object(VMTuple::new(&mut vec![]));

    let mut line_count = 0;
    let mut dir_stack = DirStack::new(None).unwrap();

    loop {
        // 开始一个新的多行输入
        let mut input_buffer = String::new();
        let mut is_multiline = false;

        // 主提示符
        let main_prompt = format!("In[{}]: ", line_count);

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
                match build_code(&input_buffer, &mut dir_stack) {
                    Ok(package) => {
                        let mut translator = IRTranslator::new(&package);
                        if translator.translate().is_ok() {
                            let result = translator.get_result();
                            match execute_ir_repl(result, &mut gc_system, &mut input_arguments) {
                                Ok(mut lambda_ref) => {
                                    let executed = lambda_ref.as_type::<VMLambda>();
                                    let result_ref = &mut executed.result;
                                    let idx = input_arguments.as_type::<VMTuple>().values.len();
                                    match try_repr_vmobject(result_ref, None) {
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
                                                    format!(
                                                        "Error: {}",
                                                        result.unwrap_err().to_string()
                                                    )
                                                    .red()
                                                );
                                            }
                                        }
                                        Err(mut e) => {
                                            println!(
                                                "{}{}",
                                                format!("Out[{}]", idx).blue().bold(),
                                                format!("<Unable to repr: {}>", e.to_string())
                                                    .red()
                                            );
                                            e.consume_ref();
                                        }
                                    }
                                    lambda_ref.drop_ref();
                                }
                                Err(mut e) => {
                                    println!(
                                        "{}",
                                        format!("Execution error: {}", e.to_string())
                                            .red()
                                            .bold()
                                            .underline()
                                    );
                                    e.consume_ref();
                                }
                            }
                        } else {
                            println!("{}", "IR translation failed.".red());
                        }
                    }
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
                println!("{}", format!("{}", err).red());
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
    // 1. Check for unbalanced brackets/braces/parens and unterminated strings
    let mut brace_count = 0; // { }
    let mut bracket_count = 0; // [ ]
    let mut paren_count = 0; // ( )
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
        } else {
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
    }

    // Incomplete if brackets are open or inside a string
    if brace_count > 0 || bracket_count > 0 || paren_count > 0 || in_string {
        return false;
    }
    // Note: A negative count means too many closing brackets, which is a syntax error,
    // but for the purpose of REPL continuation, we consider it "complete" (or rather, ready for parsing).

    // 2. Check if the input ends with a token that requires more input
    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        return true; // Empty or whitespace-only input is complete
    }

    // Use the lexer to find the last significant token
    let tokens = lexer::tokenize(trimmed_input);
    let tokens = lexer::reject_comment(&tokens); // Ignore comments

    if let Some(last_token) = tokens.last() {
        let token_str = last_token.token;

        // Check for operators that usually expect a right-hand side or subsequent input
        let is_trailing_operator = matches!(
            token_str,
            // Arithmetic, Comparison, Logical, Assignment, Access, Bitwise
            "+" | "-" | "*" | "/" | "%" | "**" |
            "==" | "!=" | "<" | ">" | "<=" | ">=" |
            "and" | "or" |
            "=" | ":=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | // Include compound assignments
            "|" | "&" | "^" | "<<" | ">>" | // Bitwise/Set operators
            "." | "->"
        );

        if is_trailing_operator {
            return false;
        }

        // Check for keywords that expect a following expression or block
        let is_trailing_keyword = matches!(
            token_str,
            "if" | "else" | "while" | "bind" | "return" | "emit" | "in" | "async" | "await"
        );

        if is_trailing_keyword {
            // Special case: `else` might appear alone temporarily but needs more
            return false;
        }

        // Check for trailing comma (usually indicates more items in lists, tuples, args)
        if token_str == "," {
            return false;
        }

        // Check for keywords that might *start* a block implicitly if followed by newline/indent
        // Example: `bind my_func =` should be incomplete. The operator check handles this.
        // Example: `if condition` should be incomplete. The keyword check handles this.
    } else {
        // If there are no tokens after trimming and removing comments, it's complete.
        return true;
    }

    // If brackets are balanced, not in a string, and the last token doesn't
    // indicate necessary continuation, consider the input complete.
    true
}
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            bytecode,
        } => {
            if let Err(e) = compile_file(&input, output, bytecode) {
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
        Commands::DisplayIR { input } => {
            match IRPackage::read_from_file(input.to_str().unwrap()) {
                Ok(package) => {
                    let ir = package.instructions;
                    let source = package.source.unwrap_or_else(|| "No source".to_string());
                    let function_ips = package.function_ips;

                    // 打印 IR 包基本信息
                    println!("{}", "IR Package Information:".bright_blue().bold());
                    println!(
                        "Source:\n{}",
                        if source == "No source" {
                            source.yellow()
                        } else {
                            source.bright_green()
                        }
                    );
                    println!("\n{}", "Function entry points:".bright_blue());

                    // 打印函数入口点信息
                    for (name, ip) in &function_ips {
                        println!("  {} => {}", name.bright_yellow(), ip);
                    }

                    println!("\n{}", "Instructions:".bright_blue().bold());

                    // 查找每条指令所属的函数
                    for (i, (debug_info, instruction)) in ir.iter().enumerate() {
                        // 检查是否是函数入口点
                        for (name, &ip) in &function_ips {
                            if ip == i {
                                println!(
                                    "\n{} {}:",
                                    "Function".bright_magenta().bold(),
                                    name.bright_cyan().bold()
                                );
                            }
                        }

                        // 显示源代码位置（如果有）
                        let pos_info = if debug_info.code_position != 0 {
                            format!("[pos: {}]", debug_info.code_position).bright_black()
                        } else {
                            "".normal()
                        };

                        // 格式化输出指令
                        let instr_str = format!(
                            "{:06}: {:<30} {}",
                            i,
                            format!("{:?}", instruction),
                            pos_info
                        );
                        println!("{}", instr_str.bright_white());
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Error reading IR file: {}", e).bright_red());
                    std::process::exit(1);
                }
            }
        }
        Commands::Translate { input, output } => {
            if let Err(e) = ir_to_bytecode_file(&input, output) {
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
        Commands::Lsp { port } => {
            // 启动 LSP 服务器
            let result = lsp::start_lsp_server(port);
            if let Err(e) = result {
                eprintln!("LSP server error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
