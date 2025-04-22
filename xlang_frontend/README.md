# XLang Frontend

`xlang_frontend` 是实验性编程语言 [XLang-Rust](https://github.com/sjrsjz/XLang-Rust) 的编译器前端库。

## 功能

该库提供了将 XLang-Rust 源代码转换为虚拟机可执行的中间表示 (IR) 所需的工具，包括：

*   **词法分析器 (Lexer):** 将源代码文本分解成一系列的词法单元 (Token)。
*   **语法分析器 (Parser):** 根据语言的语法规则，将 Token 流构建成抽象语法树 (AST)。
*   **静态分析器 (Analyzer):** 对 AST 进行分析，检查变量作用域、处理注解，并进行一些基本的静态检查。
*   **IR 生成器 (IR Generator):** 遍历 AST，生成 XLang 虚拟机的中间表示 (IR)。
*   **编译辅助:** 提供将源代码直接编译到 IR 包或字节码的接口。
*   **目录栈管理:** 用于`@compile` 时的相对路径。

## 用途

`xlang_frontend` 主要作为 XLang-Rust 编译器和解释器 (`xlang-rust` 主程序包) 的依赖项，负责处理源代码的解析和编译过程。