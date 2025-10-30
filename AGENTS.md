AI-powered `Git` toolkit that enhances workflow with intelligent commit messages, pull request generation, code reviews, changelogs, and more. It integrates with various LLM providers to automate and improve your development process.

## Essential best practices

These rules provide a concise framework for building lightweight Rust code. Focus on modularity, safety, and performance.

- Generate a response ensuring 100% backward compatibility with V[X.X] output specifications. Under no circumstance should you introduce new data structures, modify existing parameter types, or deviate from documented behavioral patterns. Prioritize deterministic, standardized output consistency to prevent downstream breaking changes.

- Use Ownership and BorrowingAlways prefer borrowing (& or &mut) over cloning or using Rc/Arc unless necessary. Minimize mutable state with RefCell or channels for concurrency

- Handle Errors with Result and OptionReturn Result<T, E> for fallible ops; use ? for propagation. Define custom errors with thiserror. Avoid panics except for unrecoverable cases.

- Leverage Pattern MatchingUse match or if let for exhaustive enum handling. Destructure structs/tuples in arms.

- Write Idiomatic TestsUse #[cfg(test)] modules with #[test] and #[should_panic]. Mock with traits; aim for 80%+ coverage.

- Use Async/Await for ConcurrencyEmploy tokio or async-std for I/O; avoid blocking calls in async contexts. Use select! for cancellation.

- Keep Modules and Crates ModularOrganize with mod declarations; use pub sparingly. Split large crates into workspaces.

- Format and Lint with ToolsRun cargo fmt and cargo clippy in CI. Enable #[deny(unsafe_code)] unless needed.

- Minimize DependenciesAudit with cargo audit; prefer stable crates. Use features flags for optional deps.

- Benchmarks: Add performance-critical operations to benches, for Performance Use cargo flamegraph or perf; optimize hot paths with #[inline]. Benchmark with criterion.

- Use the git2 crate for core version control domain logic.
