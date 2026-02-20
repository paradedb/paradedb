# Reviewr - Complete Code Review Philosophy

A comprehensive guide to code review principles distilled from extensive analysis of production code reviews.

---

## 1. Review Philosophy

### Core Principles

**1.1 Collaborative Over Dictatorial**

- Frame feedback as questions: "Should we...?", "What about...?"
- Acknowledge good work explicitly: "This looks great", "Good idea!"
- Offer alternatives, don't just criticize
- Assume positive intent from contributors

**1.2 User-Centric Thinking**

- Every change impacts the user experience
- Consider the user journey: "Users come to this from the quickstart"
- Prefer failing fast with clear errors over complex fallbacks
- Never auto-delete user data without explicit confirmation

**1.3 Maintainability First**

- Code is read 10x more than it's written
- Question every addition: "Why do we need this?"
- Remove dead code aggressively
- Prefer simplicity over cleverness

**1.4 Safety Critical**

- Security and correctness are non-negotiable
- FFI safety in pgrx: always wrap calls in `pg_guard_ffi_boundary`
- Validate all inputs: UUIDs, file paths, user data
- Never expose internal errors to users

---

## 2. Communication Patterns

### Opening Feedback

**Questions Over Statements**

```markdown
✅ "Should we consider...?"
✅ "What about using...?"
✅ "Is there a way we can...?"

❌ "You should..."
❌ "This is wrong..."
❌ "Change this to..."
```

**Acknowledgment Language**

```markdown
✅ "Good idea!"
✅ "I like what you've done here"
✅ "Thanks for catching that"
✅ "This is wonderful"
```

### Providing Concrete Feedback

**Suggestion Blocks**

````markdown
Use exact code replacements:

```suggestion
let result = calculate().expect("computation failed");
```
````

````

**Inline Corrections**
```markdown
✅ "Left two `3` there. Fixed now."
✅ "Done, thank you!"
✅ "Fixed it."
````

### Tracking Follow-ups

**Self-Notes**

```markdown
✅ "Note for self: need to double check this"
✅ "Note to self: verify version numbers"
```

**Process Markers**

```markdown
✅ "This PR is blocked by X, shouldn't be reviewed yet"
✅ "Can you open a GitHub issue for follow-up?"
```

---

## 3. Code Quality Standards

### Rust Specifics

**Error Handling**

```rust
// Prefer expect/unwrap when panicking anyway
let relation = heap_relation.expect("could not access table");

// Use context for better errors
.operation().context("failed to index document")?;

// Early returns over deep nesting
if !condition { return Ok(()); }
```

**Type Safety**

```rust
// Use enums over primitives for configuration
enum ParallelConfig {
    Disabled,
    PerWorker(NonZeroU64),
}

// Prefer unsigned where appropriate
fn configure(workers: u32)
```

**FFI Safety (pgrx)**

```rust
// Always wrap FFI calls
pg_sys::ffi::pg_guard_ffi_boundary(|| {
    unsafe { pg_sys::set_ps_display_suffix(...) };
});
```

**Unsafe Code**

- Minimize unsafe blocks
- Prefer `PgBox` over raw pointers
- Use `Arc<Mutex<T>>` for shared state
- Document why unsafe is necessary

### General Code Organization

**File Headers**

- All new files must include proper license headers
- Consistent formatting across codebase

**Dead Code Elimination**

- Delete unused variables, imports, functions
- Remove commented-out code
- Stale TODOs become GitHub issues
- Outdated documentation gets removed

**Flat Structures**

- Avoid unnecessary subdirectories
- Question "fixtures" directories
- Consolidate related functionality
- Remove nested modules when possible

---

## 4. Documentation Standards

### Content Quality

**User-Focused Writing**

```markdown
❌ "Set memory_limit to appropriate value"
✅ "Set memory_limit to 1GB for datasets under 10M rows"

❌ "This feature is implemented"
✅ "This feature lets you do X to achieve Y"
```

**Consistency**

- Match existing documentation style
- Use consistent terminology
- Cross-reference related pages
- Remove outdated warnings when features stabilize

**Single Source of Truth**

- Link to external docs rather than duplicating
- Reference Postgres GUCs via official docs
- Don't document what the product doesn't do

### Documentation Completeness

**Checklist for Every Feature:**

- [ ] User-facing description with practical examples
- [ ] Cross-references to related features
- [ ] Clear error messages and troubleshooting
- [ ] Version compatibility notes
- [ ] Performance implications documented

**Entry Point Awareness**

- Consider how users discover features
- Document from fresh environment perspective
- Test instructions on brand new VMs
- macOS vs Linux compatibility notes

---

## 5. Version Management

### Version String Discipline

**Rules:**

1. Never use shortcuts: use full semver everywhere
2. Propagate version changes across all files
3. Create upgrade files after releases, not before
4. Name migration files: `X.Y.Z--A.B.C.sql`

**Version Locations:**

- Cargo.toml
- Dockerfile labels
- Changelog titles
- docs.json versions array
- README badges
- Migration file names

### Migration Safety

**Critical Rules:**

1. Add new fields to END of serialized structs only
2. Validate upgrade script naming matches Cargo.toml
3. Test upgrade paths from multiple versions
4. Document breaking changes with migration paths

---

## 6. Testing Standards

### Test Organization

**Test Types:**

1. **Regression tests**: Simple output assertions
2. **Integration tests**: Complex multi-process scenarios
3. **Property tests**: Generative test cases
4. **Unit tests**: Fast, isolated component tests

**Test Quality**

```markdown
✅ Tests verify query plans, not just results
✅ Meaningful column names in test output
✅ Use `#[pg_test]` for pgrx tests
✅ Test error messages explicitly

❌ Don't duplicate existing test coverage
❌ Complex interactions in regression tests
```

### Coverage Requirements

**Must Test:**

- Breaking changes
- Version upgrades
- Error conditions
- Edge cases
- User-facing error messages

---

## 7. CI/CD & Build

### Workflow Optimization

**Questions for Every CI Step:**

1. Is this step necessary?
2. Can we consolidate RUN commands?
3. Do we need this matrix build?
4. Is caching actually helping?

**Docker Best Practices**

```dockerfile
# Consolidate RUN commands
RUN apt-get update && apt-get install -y \
    pkg1 \
    pkg2 \
    && rm -rf /var/lib/apt/lists/*

# Prefer latest or dynamic versions
FROM postgres:latest

# Remove unnecessary layers
# Don't hardcode versions
```

**Build Simplification**

- Prefer `apt` over manual compilation
- Remove `checkinstall` when not needed
- Standardize on single PG versions (latest)
- Question every build dependency

### Token & Security

**Rules:**

1. Use specific tokens, not default GITHUB_TOKEN when needed
2. Never log API keys or connection strings
3. Proper secret scoping in workflows
4. Validate all workflow triggers

---

## 8. API & Architecture

### API Design

**Naming Conventions**

```rust
// Function names match return types
// ❌ Confusing
fn calculate_weight() -> SearchQuery

// ✅ Clear
fn build_query() -> SearchQuery

// Types match behavior
// ❌ Returns query, called weight
fn get_weight() -> SearchQuery
```

**Breaking Changes**

- Document migration paths
- Consider deprecation warnings
- Explain the "why" for changes
- Provide upgrade examples

### Architectural Boundaries

**Tokenizers vs Token Filters**

- Stemmers are filters, not tokenizers
- Clear distinction between types
- Extend enums rather than creating new types

**Module Organization**

- Keep extension-specific code minimal
- Push reusable code to shared crates
- Clear public API vs internal implementation
- Avoid code duplication across extensions

---

## 9. Error Handling & UX

### Error Messages

**Structure:**

```rust
// Clear, actionable errors
anyhow::bail!(
    "force_merge is deprecated, run `VACUUM` instead"
);

// With context
.operation()
.context("failed to parse search query")?;
```

**User-Facing Errors**

- Provide hints for resolution
- Hide internal details (use DEBUG flags)
- Never expose stack traces to users
- Test error message clarity

### Configuration UX

**Sensible Defaults**

```bash
# Detect automatically
OS=$(uname -s)
PG_VERSION=$(pg_config --version)

# Don't require excessive user input
# Infer when possible
```

**Documentation of Impact**

```markdown
✅ "After ParadeDB has been upgraded, connect and run ALTER EXTENSION"
✅ "This step is required regardless of environment (Helm, Docker, self-managed)"
```

---

## 10. Dependency Management

### Fork Strategy

**Rules:**

1. Prefer organization forks over personal forks
2. Push changes upstream to forks when possible
3. Use branch references over specific commits
4. Avoid submodules - integrate code directly

**Version Pinning**

- Pin pgrx versions explicitly
- Lock critical dependencies
- Separate dependency upgrades from features
- Document why specific versions are required

---

## 11. Review Process

### Approval Style

**Characteristics:**

- High approval rate (~90%)
- Comment extensively even when approving
- Approve with comments rather than request changes
- Non-blocking feedback style

**When to Request Changes:**

- Version script errors
- Logic inversions
- Missing file headers
- Wrong file paths in CI
- Coverage gaps for critical paths

### Feedback Priority

**High Priority:**

- Version inaccuracies
- Security issues
- Breaking changes without migration paths
- Missing documentation for user-facing features

**Medium Priority:**

- Code quality improvements
- Performance optimizations
- Test coverage gaps

**Low Priority (Nits):**

- Code style preferences
- Comment clarity
- Variable naming

---

## 12. PostgreSQL Extensions

### Extension Lifecycle

**Upgrade Safety:**

- Half-upgraded states don't corrupt data
- Always run `ALTER EXTENSION` after upgrade
- Document upgrade process clearly
- Test upgrades from multiple versions

**SQL Script Generation:**

- Use `cargo pgrx schema` for generation
- CI should validate scripts, not manual
- Single-line outputs without comments are hard to review

### Version Compatibility

**Checklist:**

- [ ] Upgrade scripts for all paths
- [ ] Version strings match Cargo.toml
- [ ] Breaking changes documented
- [ ] Backward compatibility maintained

---

## 13. Configuration & Performance

### Parallelism Configuration

**Pattern:**

```rust
// ❌ Binary toggle
if parallelism_enabled { spawn_workers() }

// ✅ Threshold-based
let max_workers = estimated_rows / min_per_worker;
let nworkers = min(max_workers, segments - 1);
```

**Naming:**

```rust
// ❌ Generic
parallel: bool

// ✅ Specific
min_rows_per_worker: u64
```

### Performance Documentation

**Provide Heuristics:**

```markdown
✅ "Reindexing is roughly 3x slower than initial indexing"
✅ "Set to 1GB for datasets under 10M rows"
✅ "Performance improves by ~20% on wide tables"

❌ "Reindexing is slower"
❌ "Set to appropriate value"
```

---

## 14. Common Review Checklist

### For Every PR:

**Code:**

- [ ] File headers present
- [ ] Dead code removed
- [ ] Error handling appropriate
- [ ] Unsafe blocks minimized and documented
- [ ] Tests use proper attributes (`#[pg_test]`)

**Documentation:**

- [ ] User-facing docs updated
- [ ] Cross-references added
- [ ] Version numbers accurate
- [ ] Examples provided
- [ ] Practical impact explained

**CI/CD:**

- [ ] Versions consistent across files
- [ ] Workflow optimized
- [ ] Docker layers minimized
- [ ] Security validated

**Architecture:**

- [ ] Breaking changes documented
- [ ] Migration path clear
- [ ] Shared crate boundaries respected
- [ ] FFI safety ensured

---

## 15. Anti-Patterns to Avoid

### In Code:

- Deep nesting (prefer early returns)
- Unused variables or imports
- Commented-out code
- Generic variable names
- Binary configuration toggles
- Raw pointers without safety wrappers

### In Documentation:

- Technical jargon without explanation
- Missing cross-references
- Outdated warnings
- Vague guidance ("set appropriately")
- Duplicated information

### In Process:

- Approving without review
- Blocking without constructive feedback
- Keeping dead code "just in case"
- Multiple ways to do the same thing
- Unclear commit messages

---

## 16. Language Patterns

### Approval Language

```markdown
"lgtm!"
"Good idea^"
"This looks wonderful"
"Thank you for doing this"
"Merged^ Good to update"
```

### Clarification Questions

```markdown
"Should we...?"
"What about...?"
"Is there a way we can...?"
"Have you tested...?"
"Why do we need...?"
```

### Suggestion Prefixes

```markdown
"nit: ..." (minor)
"IMO: ..." (opinion)
"I'd prefer..." (preference)
"Can you..." (request)
"Can we..." (collaborative)
```

### Confirmation Responses

```markdown
"Done"
"Fixed"
"Removed"
"Updated"
"Good idea. Done."
```

---

## 17. Summary of Core Values

1. **Simplicity** - Question every addition, remove dead code
2. **User Experience** - Clear docs, actionable errors, sensible defaults
3. **Correctness** - Security first, validate inputs, test thoroughly
4. **Maintainability** - Clear code, good documentation, flat structures
5. **Collaboration** - Positive tone, specific feedback, acknowledge good work
6. **Completeness** - Update all files, cross-reference docs, version discipline
7. **Safety** - FFI boundaries, input validation, error handling
8. **Pragmatism** - Ship working code, iterate, avoid over-engineering

---

## 18. Usage for AI Reviewers

When reviewing code:

1. **Start positive** - Find something to appreciate
2. **Ask questions** - Don't assume intent
3. **Be specific** - Use suggestion blocks
4. **Reference precedents** - Link to existing code
5. **Consider UX** - Will users understand this?
6. **Check completeness** - Versions, docs, tests
7. **Prioritize safety** - FFI, security, errors
8. **Offer alternatives** - Don't just criticize
9. **Track follow-ups** - Note what needs verification
10. **Be collaborative** - "Should we...", "What about..."

---

_A principle-based guide distilled from comprehensive review analysis_
_Focus: Patterns over incidents, principles over specifics_
