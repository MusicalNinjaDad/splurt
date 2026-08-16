# Ignore this file

## Coding Standards Inferred from Staged Changes

### Language Features

- Use unstable Rust features (`try_blocks`, `exact_div`) when they improve clarity or reduce boilerplate
- Enable features explicitly at crate root

### Safety

- Isolate all `unsafe` code to dedicated wrapper modules
- Deny `unsafe_code` globally with `#![deny(unsafe_code)]`
- Forbid `unsafe_op_in_unsafe_fn` and `unsafe_attr_outside_unsafe`
- Never allow unsafe code outside explicitly designated modules

### Error Handling

- Prefer `try {}` blocks for cleaner error propagation chains
- Use `.ok()?` pattern within try blocks to flatten `Result<Option<T>>`
- Return `Option<&T>` for cached data that may not exist
- Return `io::Result<T>` for operations involving I/O
- Let callers handle unwrapping/cloning: `disc_mut().cover_art()?.clone().unwrap()`

### API Design

- Provide both immutable and mutable accessors (`disc()`, `disc_mut()`)
- Return references to internal state rather than cloning
- Favor lazy initialization over eager loading
- Cache expensive operations in struct fields
- Expect callers to handle Option types explicitly

### Caching

- Lazy-load expensive resources (cover art on first access)
- Eager-load essential metadata (MusicBrainz in constructor)
- Store cached data directly in struct fields (`Option<Vec<u8>>`)
- Avoid interior mutability patterns (RefCell) when mutable access is available

### Code Organization

- Use module system to separate concerns
- Define traits for shared behavior
- Re-export types at crate root for convenience
- Group related types and constants together

### Documentation

- Document module purpose and domain concepts at module level
- Document behavior and invariants for public items
- Use `///` for all documentation comments
- Include domain-specific terminology in docs

### Debugging

- Use `dbg!` for critical values during development
- Use `debug_assert` to verify invariants
- Keep debug statements in place

### Naming

- Use domain-specific terminology (Disc, Track, CdTime)
- Use clear, descriptive names for types and methods
- Follow Rust naming conventions consistently
