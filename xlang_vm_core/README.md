# XLang VM Core

`xlang_vm_core` is the core virtual machine and runtime library for the experimental programming language [XLang-Rust](https://github.com/sjrsjz/XLang-Rust).

## Features

This library provides the infrastructure required for executing XLang-Rust language, including:

*   **Virtual Machine (VM):** Responsible for interpreting and executing compiled XLang bytecode.
*   **Garbage Collector (GC):** Automatically manages memory, reclaiming objects that are no longer in use.
*   **Type System:** Defines and operates on XLang's built-in data types (such as integers, floats, strings, tuples, Lambda, etc.).
*   **Execution Context:** Manages scopes, stack frames, and variables.
*   **Intermediate Representation (IR):** Defines the intermediate code format between the compiler and VM.
*   **IR Translator:** Translates IR into bytecode executable by the VM.
*   **Foreign Function Interface (FFI):** Allows XLang code to call dynamic libraries written in C.
*   **Built-in Functions:** Provides core built-in functionality (such as `print`, `len`, type conversions, etc.).

## Usage

`xlang_vm_core` primarily serves as a dependency for the XLang-Rust compiler and interpreter (`xlang-rust` main package), providing the actual code execution capability.