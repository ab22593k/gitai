AI System Instruction: Rust Code Generation and Editing.

## Code Generation Process

Mandatory Chain-of-Thought (CoT) Analysis: Before generating or modifying any code, you must first output a CoT analysis plan. This plan will explicitly detail the required changes, identify the relevant architectural patterns, security mandates, and naming conventions to be applied, and outline the implementation steps. This process is mandatory to ensure adherence to production standards and minimize errors.

## Low-Latency Architectural Patterns

- Asynchronous I/O: Utilize async/await with the Tokio runtime for all non-blocking I/O operations, particularly for network services, to prevent thread blocking and improve concurrency.

- Parallel Fan-Out: When querying multiple independent data sources, use constructs like tokio::join! to execute I/O-bound requests concurrently, minimizing compounded latency.

- Data and Task Parallelism: For CPU-bound tasks, leverage libraries like rayon to convert sequential operations into parallel ones, maximizing the use of multi-core processors.

- Data-Centric Latency Reduction: Employ data colocation, partitioning, and caching strategies to minimize data access delays.

- Data Colocation and Partitioning: Design systems to colocate compute and data. Implement shard-per-core architectures where incoming work is partitioned on a per-CPU basis, with each core managing dedicated memory and I/O to avoid context switches and locking.

- Synchronization and Memory Management: Minimize contention and memory allocation overhead in performance-critical paths.

- Wait-Free Synchronization: In highly contended components (e.g., queues, state machines), use lock-free data structures (e.g., from crossbeam) instead of traditional mutexes to eliminate locking overhead and reduce tail latency.

- Memory Allocation: Avoid frequent memory allocations in hot code paths. Utilize memory pooling and pre-allocation for buffers to reduce allocation overhead.

## Security and Compliance Mandates

- Input Validation: All external input from any source (user, network, file system) must be treated as untrusted and be strictly validated and sanitized before use.

- Validate against expected formats, lengths, types, and ranges.
  Use Rust's type system and pattern matching to enforce data integrity and handle invalid data gracefully.

## Data Structures & Types

&str is preferred for immutable, borrowed string data (especially in function parameters) to avoid unnecessary allocations. String is used only when ownership or mutability is required. RF: `String` vs `&str`
structs group related data fields, and enums represent types with multiple possible variants. RF: `struct` vs `enum`
Vec<T> is for general-purpose dynamic arrays. std::collections::VecDeque<T> is specifically for efficient push/pop at both ends (e.g., queues). RF: `Vec` vs `VecDeque`
The #[derive(TraitName)] attribute is used for automatic implementation of common traits like Debug, Clone, PartialEq, and serde::Serialize/Deserialize. Debug MUST always be derived for custom types. RF: `derive` traits

## Performance Optimization

`String::with_capacity()` should be used when the final string size is known or estimable to pre-allocate buffers and avoid reallocations.
`String::with_capacity` Zero-copy operations are preferred by passing data via slices (&str, &[T]) instead of owned types to avoid unnecessary allocations and copies. RF: Zero-Copy Slices

## Benchmarking

Performance analysis MUST use the criterion crate for statistical rigor. RF: `criterion` Benchmarking
Inputs and/or outputs of functions being benchmarked MUST be wrapped with criterion::black_box() to prevent compiler optimization that could lead to inaccurate results. RF: Purpose of `criterion::black_box`

## MUST USE Formating

When generating Rust code that uses macros like format!, println!, or write!, always prefer inlining variables if they have the same name as the format placeholder. Do not use an empty {} placeholder with a separate variable argument if the variable name can be used directly within the braces. This is to adhere to the Clippy lint uninlined_format_args and use modern Rust syntax

Original (Incorrect style):

```rust
format!("{} {}", marker, checkbox)
```

Corrected (Idiomatic Rust 2021+ style):

```rust
format!("{marker} {checkbox}")
```
