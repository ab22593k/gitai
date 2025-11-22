AI System Instruction: Rust Code Generation and Editing.

## 1.0 Code Generation Process

Mandatory Chain-of-Thought (CoT) Analysis: Before generating or modifying any code, you must first output a CoT analysis plan. This plan will explicitly detail the required changes, identify the relevant architectural patterns, security mandates, and naming conventions to be applied, and outline the implementation steps. This process is mandatory to ensure adherence to production standards and minimize errors.

## 2.0 Low-Latency Architectural Patterns

- Asynchronous I/O: Utilize async/await with the Tokio runtime for all non-blocking I/O operations, particularly for network services, to prevent thread blocking and improve concurrency.

- Parallel Fan-Out: When querying multiple independent data sources, use constructs like tokio::join! to execute I/O-bound requests concurrently, minimizing compounded latency.

- Data and Task Parallelism: For CPU-bound tasks, leverage libraries like rayon to convert sequential operations into parallel ones, maximizing the use of multi-core processors.

- Data-Centric Latency Reduction: Employ data colocation, partitioning, and caching strategies to minimize data access delays.

- Data Colocation and Partitioning: Design systems to colocate compute and data. Implement shard-per-core architectures where incoming work is partitioned on a per-CPU basis, with each core managing dedicated memory and I/O to avoid context switches and locking.

- Caching Strategies: Implement caching (e.g., cache-aside, read-through) for frequently accessed, computationally expensive data. The cache must have a clearly defined replacement policy (e.g., LRU, TTL).

- Synchronization and Memory Management: Minimize contention and memory allocation overhead in performance-critical paths.

- Wait-Free Synchronization: In highly contended components (e.g., queues, state machines), use lock-free data structures (e.g., from crossbeam) instead of traditional mutexes to eliminate locking overhead and reduce tail latency.

- Memory Allocation: Avoid frequent memory allocations in hot code paths. Utilize memory pooling and pre-allocation for buffers to reduce allocation overhead.

## 3.0 Security and Compliance Mandates

- Input Validation: All external input from any source (user, network, file system) must be treated as untrusted and be strictly validated and sanitized before use.

- Validate against expected formats, lengths, types, and ranges.
  Use Rust's type system and pattern matching to enforce data integrity and handle invalid data gracefully.

- Minimize unsafe Code: The use of unsafe blocks is heavily discouraged. When unavoidable, it must be minimized to the smallest possible scope, accompanied by a detailed comment explaining the necessity and the invariants that must be upheld for safety.

- Dependency Management: Keep all dependencies updated. Before committing, run cargo audit to check for known vulnerabilities.

## Data Structures & Types

&str is preferred for immutable, borrowed string data (especially in function parameters) to avoid unnecessary allocations. String is used only when ownership or mutability is required. RF: `String` vs `&str`
structs group related data fields, and enums represent types with multiple possible variants. RF: `struct` vs `enum`
Vec<T> is for general-purpose dynamic arrays. std::collections::VecDeque<T> is specifically for efficient push/pop at both ends (e.g., queues). RF: `Vec` vs `VecDeque`
The #[derive(TraitName)] attribute is used for automatic implementation of common traits like Debug, Clone, PartialEq, and serde::Serialize/Deserialize. Debug MUST always be derived for custom types. RF: `derive` traits

## Performance Optimization

All performance benchmarks and production artifacts MUST be compiled with cargo build --release for compiler optimizations. RF: `--release` Builds
String::with_capacity() should be used when the final string size is known or estimable to pre-allocate buffers and avoid reallocations. RF: `String::with_capacity`
Zero-copy operations are preferred by passing data via slices (&str, &[T]) instead of owned types to avoid unnecessary allocations and copies. RF: Zero-Copy Slices

## Benchmarking

Performance analysis MUST use the criterion crate for statistical rigor. RF: `criterion` Benchmarking
Inputs and/or outputs of functions being benchmarked MUST be wrapped with criterion::black_box() to prevent compiler optimization that could lead to inaccurate results. RF: Purpose of `criterion::black_box`

## Testing

Unit tests MUST be co-located with their source code within a #[cfg(test)] mod tests { ... } block. RF: Unit Tests and `#[cfg(test)]`
Individual test functions MUST be annotated with #[test]. RF: `#[test]`
assert!, assert_eq!, and assert_ne! macros are used to verify test conditions. RF: `assert!`
Public API usage examples MUST be written as documentation tests inside /// `rust ... ` blocks to keep documentation synchronized with code. RF: Documentation Tests
Raw string literals (e.g., r#""#) are used for multi-line strings or strings with special characters (like JSON or regex) to improve readability and avoid complex escaping. RF: Raw Strings `r#""#`

## Memory Safety

Rust's ownership and borrowing system is the primary defense against memory vulnerabilities. Safe Rust MUST be preferred. RF: Embrace Safe Rust and Minimize `unsafe` Code
Any unsafe block or function MUST be minimized in scope and accompanied by a comment justifying its necessity and documenting the safety invariants being manually upheld. RF: Justification for `unsafe` and Safety Invariants

## API Security

Public APIs MUST be designed to minimize the exposure of mutable state. RF: Designing Secure Rust APIs
All FFI functions that dereference raw pointers or perform other potentially memory-unsafe operations MUST be marked unsafe and wrapped in a safe, high-level Rust API. RF: FFI and Raw Pointers
