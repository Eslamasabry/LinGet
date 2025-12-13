# Contributing to LinGet

First off, thanks for taking the time to contribute! LinGet is a community-driven project and we value your input.

## How Can I Contribute?

### Reporting Bugs

This section guides you through submitting a bug report for LinGet. Following these guidelines helps maintainers and the community understand your report, reproduce the behavior, and find related reports.

- **Use a clear and descriptive title** for the issue to identify the problem.
- **Describe the exact steps to reproduce the problem** in as much detail as possible.
- **Describe the behavior you observed** after following the steps and point out what exactly is the problem with that behavior.
- **Explain which behavior you expected to see instead** and why.
- **Include screenshots and animated GIFs** which show you following the described steps and clearly demonstrate the problem.

### Suggesting Enhancements

This section guides you through submitting an enhancement suggestion for LinGet, including completely new features and minor improvements to existing functionality.

- **Use a clear and descriptive title** for the issue to identify the suggestion.
- **Provide a step-by-step description of the suggested enhancement** in as much detail as possible.
- **Explain why this enhancement would be useful** to most LinGet users.

### Pull Requests

The process described here has several goals:

- Maintain LinGet's quality
- Fix problems that are important to users
- Engage the community in working toward the best possible LinGet

Please follow these steps to have your contribution considered by the maintainers:

1.  **Fork the repository** and create your branch from `main`.
2.  **Clone the repository** to your local machine.
3.  **Create a new branch** for your fix or feature.
4.  **Test your changes** to ensure they work as expected.
5.  **Make sure your code lints and compiles** (`cargo check`, `cargo clippy`).
6.  **Submit a pull request**!

## Development Setup

1.  Install Rust and system dependencies (GTK4, Libadwaita).
2.  Clone the repo: `git clone https://github.com/linget/linget`
3.  Run the app: `cargo run`

## Coding Style

- We use standard Rust formatting (`cargo fmt`).
- Please ensure no warnings are left in the build output (`cargo check` should be clean).
- Follow the existing project structure:
    - `src/ui/`: UI components and logic
    - `src/backend/`: Package manager implementations
    - `src/models/`: Data structures

## License

By contributing, you agree that your contributions will be licensed under its GPL-3.0 License.
