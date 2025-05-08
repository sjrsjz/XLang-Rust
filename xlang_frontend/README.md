# XLang Frontend

`xlang_frontend` is the frontend library for the experimental programming language [XLang-Rust](https://github.com/sjrsjz/XLang-Rust).

## Features

This library provides the tools needed to convert XLang-Rust source code into an intermediate representation (IR) executable by the virtual machine, including:

*   **Lexer:** Breaks down the source code text into a series of tokens.
*   **Parser:** Constructs an abstract syntax tree (AST) from the token stream according to the language's grammar rules.
*   **Static Analyzer:** Analyzes the AST, checks variable scopes, processes annotations, and performs some basic static checks.
*   **IR Generator:** Traverses the AST to generate the intermediate representation (IR) for the XLang virtual machine.
*   **Compilation Helpers:** Provides interfaces for directly compiling source code to IR packages or bytecode.
*   **Directory Stack Management:** Used for relative paths during `@compile`.

## Usage

`xlang_frontend` primarily serves as a dependency for the XLang-Rust compiler and interpreter (`xlang-rust` main package), responsible for handling source code parsing and compilation processes.