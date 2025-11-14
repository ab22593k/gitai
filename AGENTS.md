A comprehensive guide to fact-grounded coding practices, ensuring modern, responsive, and secure development.

I. Project-Specific Naming Conventions

These conventions are grounded in specific "RF" (Reference) documents:

    Functions, Variables, Modules, and Files: MUST use snake_case (e.g., my_function, local_variable, my_module, my_module.rs). RF: Naming Conventions for Functions, Variables, and Modules
    Structs, Enums, Traits, and Type Parameters: MUST use PascalCase (e.g., MyStruct, MyEnum, MyTrait, T). RF: Naming Conventions for Structs, Enums, Traits, and Type Parameters
    Macros: MUST use snake_case! with an exclamation mark suffix (e.g., println!, assert_eq!). RF: Naming Conventions for Macros
    Lifetimes: MUST use a lowercase, typically single-letter, name prefixed with an apostrophe (e.g., 'a, 'static). RF: Naming Conventions for Lifetimes
    C-FFI Functions: Functions exposed to C via FFI should retain C-style snake_case for compatibility (e.g., ngx_http_calculator_handler). Internal Rust helper functions within FFI wrappers MUST follow standard Rust conventions. RF: Naming Conventions for Functions and Methods

II. Approved Architectural Patterns
Modularity

    Code should be organized into logical modules using mod module_name; declarations and corresponding module_name.rs files. RF: Rust Module Organization Fundamentals
    Item visibility is controlled by pub (public APIs), pub(crate) (internal crate-wide access), and default private visibility for encapsulation. RF: Visibility Keywords
    pub use crate::path::ToItem; should be used to re-export items, creating a simplified public API and avoiding deep nesting. RF: Best Practices for `pub use` for Clean APIs

Error Handling

    std::result::Result<T, E> is mandatory for functions that may encounter recoverable errors. RF: `Result`: The Core of Recoverable Error Handling
    Custom enum Error { ... } types should represent domain-specific failure modes. These enums MUST implement std::fmt::Display (for user messages) and std::fmt::Debug (for developer logging). RF: Custom Enum Error Types
    The ? operator is used to propagate Result errors up the call stack, reducing boilerplate and improving readability. RF: `?` Operator for Error Propagation
    panic!, .expect(), and .unwrap() are reserved for unrecoverable errors (bugs, violated invariants). .expect() is preferred over .unwrap() for descriptive panic messages. RF: `panic!`, `expect`, `unwrap`

Data Structures & Types

    &str is preferred for immutable, borrowed string data (especially in function parameters) to avoid unnecessary allocations. String is used only when ownership or mutability is required. RF: `String` vs `&str`
    structs group related data fields, and enums represent types with multiple possible variants. RF: `struct` vs `enum`
    Vec<T> is for general-purpose dynamic arrays. std::collections::VecDeque<T> is specifically for efficient push/pop at both ends (e.g., queues). RF: `Vec` vs `VecDeque`
    The #[derive(TraitName)] attribute is used for automatic implementation of common traits like Debug, Clone, PartialEq, and serde::Serialize/Deserialize. Debug MUST always be derived for custom types. RF: `derive` traits

Foreign Function Interfaces (FFI) & WebAssembly (Wasm)
C FFI

    Rust functions exposed to C MUST use #[no_mangle] pub unsafe extern "C" fn for a stable C ABI and to prevent name mangling. RF: `no_mangle` and `extern "C"`
    Types from the libc crate (e.g., c_char, c_int) MUST be used for function arguments and return types for C compatibility. RF: `libc` Types
    Raw pointers received from C MUST be checked for null (ptr.is_null()) within an unsafe block before dereferencing. RF: Null Checks
    Null-terminated C strings (*const c_char) are converted to Rust strings using std::ffi::CStr::from_ptr() followed by .to_str(), which includes UTF-8 validation. RF: `CStr` and `CString`
    Memory ownership at FFI boundaries MUST be explicitly managed. The allocator is responsible for deallocation. Dedicated Rust functions for freeing memory allocated in Rust but passed to C are required. RF: Memory Ownership

Python FFI (PyO3)

    Rust crates intended as Python modules MUST define an entry point function annotated with #[pymodule]. RF: Defining Python Modules and Functions
    Rust functions exposed to Python MUST be annotated with #[pyfunction] and registered within the #[pymodule] block. RF: Defining Python Modules and Functions
    The Global Interpreter Lock (GIL) MUST be released using py.allow_threads(|| { ... }) for computationally intensive, non-Python-interacting Rust code to enable true parallelism. RF: Global Interpreter Lock (GIL) Release

Asynchronous Programming

    Rust's async/await syntax is employed for all concurrent and non-blocking I/O operations. RF: `async/await` with Tokio
    All asynchronous tasks MUST run within an appropriate async runtime. tokio is used for application-level services, and tokio_wasi for WASI contexts. The runtime's event loop MUST not be blocked. RF: `async/await` with Tokio

Performance Optimization

    All performance benchmarks and production artifacts MUST be compiled with cargo build --release for compiler optimizations. RF: `--release` Builds
    String::with_capacity() should be used when the final string size is known or estimable to pre-allocate buffers and avoid reallocations. RF: `String::with_capacity`
    Zero-copy operations are preferred by passing data via slices (&str, &[T]) instead of owned types to avoid unnecessary allocations and copies. RF: Zero-Copy Slices

Benchmarking

    Performance analysis MUST use the criterion crate for statistical rigor. RF: `criterion` Benchmarking
    Inputs and/or outputs of functions being benchmarked MUST be wrapped with criterion::black_box() to prevent compiler optimization that could lead to inaccurate results. RF: Purpose of `criterion::black_box`

Testing

    Unit tests MUST be co-located with their source code within a #[cfg(test)] mod tests { ... } block. RF: Unit Tests and `#[cfg(test)]`
    Individual test functions MUST be annotated with #[test]. RF: `#[test]`
    assert!, assert_eq!, and assert_ne! macros are used to verify test conditions. RF: `assert!`
    Public API usage examples MUST be written as documentation tests inside /// ```rust ... ``` blocks to keep documentation synchronized with code. RF: Documentation Tests
    Raw string literals (e.g., r#""#) are used for multi-line strings or strings with special characters (like JSON or regex) to improve readability and avoid complex escaping. RF: Raw Strings `r#""#`

III. Defined Security Practices
Memory Safety

    Rust's ownership and borrowing system is the primary defense against memory vulnerabilities. Safe Rust MUST be preferred. RF: Embrace Safe Rust and Minimize `unsafe` Code
    Any unsafe block or function MUST be minimized in scope and accompanied by a comment justifying its necessity and documenting the safety invariants being manually upheld. RF: Justification for `unsafe` and Safety Invariants

Input Validation

    Strict input validation and sanitization MUST be implemented at all external system boundaries (FFI, Wasm, HTTP, CLI). RF: Input Validation Best Practices
    UTF-8 encoding for all string inputs from untrusted sources MUST be explicitly validated using std::str::from_utf8. RF: UTF-8 Null Checks
    Rigorous null checks MUST be performed on all raw pointers received via FFI before dereferencing. RF: FFI Null Checks
    The Result type with specific custom error enums MUST be used to clearly signal and handle validation failures. RF: FFI and Result Custom Error Types

API Security

    Public APIs MUST be designed to minimize the exposure of mutable state. RF: Designing Secure Rust APIs
    All FFI functions that dereference raw pointers or perform other potentially memory-unsafe operations MUST be marked unsafe and wrapped in a safe, high-level Rust API. RF: FFI and Raw Pointers

IV. Chain-of-Thought Approach for Planning Every Change

All code generation and editing tasks MUST be preceded by a structured planning process to ensure deliberate, well-designed changes aligned with project standards. RF: Structured Code Change Process Best Practices

    Goal Definition: Precisely define functional and non-functional objectives. RF: Software Development Planning Methodologies
    Impact Analysis: Identify all system components (Rust code, FFI, Wasm modules) affected by or interacting with the change. RF: Change Request and Impact Analysis
    Architectural Design: Outline high-level design, including module organization, data flow, concurrency strategy, and interaction points. RF: Code Structure and Organization
    Data Modeling & Types: Define or modify necessary Rust structs, enums, and types, considering ownership semantics (T, &T, &mut T) and lifetimes.
    API & Function Signature Design: Design explicit function signatures, including appropriate return types (Result<T, E>, Option<T>) and necessary FFI/Wasm attributes.
    Error Handling Strategy: Detail how potential errors will be handled and propagated using defined error types and patterns.
    Security & Safety Analysis:
        Unsafe Code: Identify required unsafe blocks or FFI calls and document manually upheld safety invariants. RF: Security
        Input Validation: Plan specific validation checks for all external inputs.
        Resource Management: Detail management of memory, file handles, network connections, etc.
    Implementation Outline: Decompose tasks into logical steps, identifying specific Rust constructs and libraries.
    Naming & Convention Adherence: Verify that all new names strictly adhere to project naming conventions. RF: Meaningful Naming
    Testing & Verification Plan: Outline a comprehensive testing strategy, including unit tests, documentation tests, and benchmarks. RF: Test-Driven Development (TDD) and Automated Testing
