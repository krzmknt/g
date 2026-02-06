# Project Guidelines

## Development Methodology

### Test-Driven Development (TDD)

- Follow the strict **Red-Green-Refactor** cycle:
  1. **Red**: Write a failing test first
  2. **Green**: Write the minimum code to make the test pass
  3. **Refactor**: Improve the code while keeping tests green
- No production code without a corresponding test
- Tests must be written before implementation, not after
- Maintain high test coverage for all business logic

### Domain-Driven Design (DDD)

- Structure code around the business domain
- Use ubiquitous language consistently across code and documentation
- Apply tactical patterns:
  - **Entities**: Objects with identity
  - **Value Objects**: Immutable objects without identity
  - **Aggregates**: Consistency boundaries
  - **Repositories**: Data access abstraction
  - **Domain Services**: Stateless domain operations
  - **Application Services**: Use case orchestration
- Separate concerns into layers:
  - Domain Layer (core business logic, no external dependencies)
  - Application Layer (use cases, orchestration)
  - Infrastructure Layer (external services, persistence)
  - Presentation Layer (API, UI)
- Keep domain logic free from framework dependencies

## Language & Documentation

- All code comments, documentation, and commit messages must be written in **English**.

## Git Workflow

### Branch Strategy

- **Direct commits to `main` branch are strictly prohibited.**
- Always create a feature branch from `main` before making changes:
  ```
  feat/[descriptive-name]
  ```
- Branch naming should reflect the task or issue being addressed.

### Branch Creation Process

1. Checkout `main` branch
2. Pull latest changes from remote
3. Create and checkout new feature branch
4. Commits to feature branches do not require approval

### Code Quality

- Remove all debug/investigation log statements before committing.
- Keep commits atomic and focused on a single change.

## Package Manager

- **Use `pnpm` exclusively.**
- Do not use `npm` or `yarn`.

---

# Allowed Commands

The following commands can be executed **without approval**:

## GitHub CLI (Read-only)

- `gh run list`
- `gh run view`
- `gh pr list`
- `gh pr view`
- `gh issue list`
- `gh issue view`
- `gh repo view`

## Git (Read-only)

- `git status`
- `git log`
- `git diff`
- `git branch`
- `git fetch`
- `git remote -v`

## Git (Write - Feature branches only)

- `git checkout -b feature/*`
- `git checkout main`
- `git checkout feature/*`
- `git add`
- `git commit`
- `git push origin feature/*`
- `git pull origin main`

## Serena

- List Memories
- Onboarding
- List Dir
- Find File
- Find Symbol
- Search For Pattern
- Write Memory

## pnpm (Read-only / Safe)

- `pnpm list`
- `pnpm why`
- `pnpm outdated`
- `pnpm test`
- `pnpm lint`
- `pnpm typecheck`
- `pnpm build`

## Cargo (Read-only / Safe)

cargo check
cargo build
cargo test
cargo clippy
cargo fmt --check
cargo doc
cargo tree
cargo outdated
cargo audit

---

# Prohibited Actions

The following actions require explicit approval:

- Pushing to `main` branch
- Force pushing (`git push -f`)
- Deleting branches
- Installing/removing dependencies (`pnpm add`, `pnpm remove`)
- Modifying CI/CD configurations
- Modifying environment variables or secrets
- Running database migrations
