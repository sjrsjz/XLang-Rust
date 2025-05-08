# XLang-Rust

XLang-Rust is a Rust implementation of the XLang programming language, designed to provide a cross-platform, experimental, dynamically strong-typed programming environment. It executes scripts through a virtual machine and offers a unique set of language features.

## Features

*   **Dynamically Strong Typed**: Variable types are determined at runtime, but type errors raise exceptions.
*   **Expression-Based**: Statements are sequences of expressions, with the result of the last expression being the statement's result. The syntax style is similar to Rust.
*   **Lambda Core**: Functions are defined and called entirely using Lambda functions, allowing Lambdas to cache their last call's parameters and return value.
*   **Tuples**: The primary ordered collection type, which are mutable and support indexing, key-value pairs, and named parameters.
*   **Single-Threaded Async Tasks**: Implements cooperative concurrency through `async`/`await` for managing tasks that interleave execution on a single thread (note: behavior differs from traditional suspendable/resumable coroutines).
*   **Non-Local Control Flow**: Uses `boundary`/`raise` to implement non-local jumps more powerful than `return`.
*   **Object Binding**: Simulates object-oriented behavior through the `bind` keyword.
*   **Alias System**: Attaches static aliases to objects using `::`.
*   **Rich Built-in Types**: Includes integers, floats, booleans, strings, byte sequences, ranges, key-value pairs, named parameters, and more.
*   **Toolchain**: Provides REPL (`repl`), compiler (`compile` to `.xir` or `.xbc`), runner (`run`), IR viewer (`display-ir`), bytecode translator (`translate`), and LSP server (`lsp`).

## Status

XLang-Rust is currently in an **experimental phase**. Its design incorporates some unique concepts and behaviors that may differ from other mainstream languages. Feedback and experimentation are welcome.

## Quick Start

Assuming the compiler executable is named `xlang-rust`:

1.  **Run a Script File**:
    ```bash
    xlang-rust run your_script.x
    ```
    You can also run intermediate code (`.xir`) or bytecode (`.xbc`) files.

2.  **Compile to Bytecode**:
    ```bash
    xlang-rust compile your_script.x -b -o your_script.xbc
    ```
    (`-b` indicates compiling to bytecode, `-o` specifies the output file)

3.  **Start the REPL**:
    ```bash
    xlang-rust repl
    ```

## Documentation

For detailed language specifications and feature introductions, please refer to: [Language Documentation](./doc/doc.pdf)