# Contributing to Velora

Welcome! We're excited you're interested in contributing to **Velora**, a modular, high-performance Rust blockchain for EVM-compatible smart contracts.

## 🚀 How to Contribute

1.  **Fork the repository** and create a new branch from `main`:
    ```bash
    git checkout -b feature/your-feature-name
    ```
2.  **Make your changes.** Ensure that:
    *   The code compiles and passes all tests.
    *   The code is formatted and linted according to the project style.
    *   You have added or updated relevant documentation.
3.  **Run the pre-commit hooks** to ensure your changes meet our quality standards:
    ```bash
    pre-commit run --all-files
    ```
4.  **Commit your changes** using the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) format.
5.  **Submit a Pull Request (PR)** with a clear description of your changes and why they are needed.

## 📐 Code Style & Quality

*   All Rust code must pass `cargo fmt` and `cargo clippy` with no warnings.
*   Avoid using `unwrap()` or `expect()` in core logic. Use `thiserror` or `anyhow` for error handling.
*   Use meaningful variable names and a clear, modular design.
*   Write unit and integration tests for all new logic.

## ✅ Testing Strategy

Run the full test suite with:
```bash
make test
```

## ✍️ Commit Message Format (Conventional Commits)

We follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard. Your commit messages should be structured as follows:

```
<type>(<scope>): <short summary>

[optional body]

[optional footer]
```

**Example:**
```
feat(txpool): add support for priority gas sorting

This commit introduces a new priority queue in the transaction pool
that sorts pending transactions by their gas price, ensuring that
higher-fee transactions are processed first.
```

**Valid `type` values:** `feat`, `fix`, `refactor`, `docs`, `chore`, `test`, `style`, `ci`, `perf`.

## 🧰 Tooling & Workflow

*   Run `make` commands to build, test, and lint the project.
*   We use `pre-commit` to automatically format and lint code before committing.

🙏 **Thank You** for helping make Velora better! 🚀
