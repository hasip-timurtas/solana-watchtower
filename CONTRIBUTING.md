# Contributing to Solana Watchtower

Thank you for your interest in contributing to Solana Watchtower! We welcome contributions from the community and are grateful for your help in making this project better.

## 🤝 How to Contribute

### Reporting Issues

If you find a bug or have a suggestion for improvement:

1. **Check existing issues** to avoid duplicates
2. **Create a detailed issue** with:
   - Clear description of the problem or suggestion
   - Steps to reproduce (for bugs)
   - Expected vs actual behavior
   - Environment details (OS, Rust version, etc.)
   - Relevant logs or screenshots

### Feature Requests

For new features:

1. **Open an issue** first to discuss the proposal
2. **Explain the use case** and why it would be valuable
3. **Consider the scope** - start with smaller, focused features
4. **Be prepared to implement** or help with implementation

## 🛠️ Development Setup

### Prerequisites

- **Rust**: 1.80+ (latest stable recommended)
- **Git**: For version control
- **Docker**: Optional, for testing containerized builds

### Local Setup

```bash
# Fork and clone the repository
git clone https://github.com/YOUR_USERNAME/solana-watchtower.git
cd solana-watchtower

# Create a development branch
git checkout -b feature/your-feature-name

# Install dependencies and build
cargo build

# Run tests to ensure everything works
cargo test

# Check code formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

### Project Structure

```
solana-watchtower/
├── crates/
│   ├── cli/           # Command-line interface
│   ├── engine/        # Core monitoring engine and rules
│   ├── subscriber/    # Solana WebSocket client
│   ├── notifier/      # Notification channels
│   └── dashboard/     # Web dashboard
├── configs/           # Configuration examples
├── docker/           # Docker deployment
├── examples/         # Usage examples
└── scripts/          # Development scripts
```

## 🔧 Development Guidelines

### Code Style

- **Format**: Use `cargo fmt` for consistent formatting
- **Linting**: Address all `cargo clippy` warnings
- **Comments**: Add meaningful comments for complex logic
- **Documentation**: Include doc comments for public APIs

### Testing

- **Unit Tests**: Write tests for new functionality
- **Integration Tests**: Add end-to-end tests when appropriate
- **Test Coverage**: Aim for good test coverage of core logic
- **Test Data**: Use realistic but anonymized test data

### Commit Messages

Follow conventional commit format:

```
feat: add new monitoring rule for NFT transfers
fix: resolve WebSocket connection timeout issue
docs: update configuration examples
test: add unit tests for alert batching
refactor: improve error handling in subscriber
```

### Branch Naming

Use descriptive branch names:
- `feature/rule-engine-improvements`
- `fix/websocket-reconnection`
- `docs/api-documentation`
- `refactor/notification-system`

## 📝 Pull Request Process

### Before Submitting

1. **Ensure tests pass**: `./scripts/run-tests.sh`
2. **Update documentation** if needed
3. **Add tests** for new functionality
4. **Check security implications** of changes
5. **Verify Docker builds** work if relevant

### PR Requirements

- **Clear description** of changes and motivation
- **Link to related issues** using `Fixes #123` or `Closes #123`
- **Screenshots** for UI changes
- **Breaking changes** clearly documented
- **Security considerations** noted if applicable

### Review Process

1. **Automated checks** must pass (CI/CD pipeline)
2. **Code review** by maintainers
3. **Security review** for sensitive changes
4. **Documentation review** for user-facing changes
5. **Final approval** and merge

## 🛡️ Security Guidelines

### Security-Sensitive Changes

For changes affecting security:

- **Follow responsible disclosure** for vulnerabilities
- **Review dependencies** for known issues
- **Consider attack vectors** and edge cases
- **Update security documentation** as needed
- **Get security review** from maintainers

### Dependency Management

- **Minimize new dependencies** - justify additions
- **Keep dependencies updated** for security patches
- **Review dependency licenses** for compatibility
- **Document security implications** of dependency changes

## 🎯 Areas for Contribution

### High Priority

- **Custom monitoring rules** for specific DeFi protocols
- **Additional notification channels** (Microsoft Teams, PagerDuty)
- **Performance optimizations** for high-volume monitoring
- **Dashboard improvements** and new features
- **Documentation and examples** for complex setups

### Medium Priority

- **Integration tests** for notification channels
- **Monitoring rule templates** for common patterns
- **Configuration validation** improvements
- **Error handling** and recovery mechanisms
- **Metrics and observability** enhancements

### Good First Issues

- **Documentation improvements** and typo fixes
- **Example configurations** for new use cases
- **Unit test additions** for existing code
- **Code cleanup** and refactoring
- **Docker configuration** improvements

## 📚 Resources

### Documentation

- [Architecture Overview](./docs/architecture.md) (if exists)
- [Security Policy](./SECURITY.md)
- [Docker Deployment](./docker/README.md)
- [Configuration Examples](./examples/README.md)

### Development Tools

- **IDE Setup**: VS Code with Rust-analyzer recommended
- **Debugging**: Use `RUST_LOG=debug` for verbose logging
- **Testing**: `cargo test` for unit tests, Docker for integration
- **Profiling**: `cargo-flamegraph` for performance analysis

### Community

- **GitHub Discussions**: For general questions and ideas
- **Issues**: For bug reports and feature requests
- **Pull Requests**: For code contributions
- **Security**: Use security@hasiptimurtas.com for vulnerabilities

## 🏷️ Labels and Project Management

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation improvements
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `security`: Security-related issue
- `performance`: Performance improvements

### Priority Labels

- `priority/critical`: Must be fixed immediately
- `priority/high`: Should be fixed soon
- `priority/medium`: Normal priority
- `priority/low`: Can be addressed later

## ✅ Definition of Done

For a contribution to be considered complete:

- [ ] Code is implemented and tested
- [ ] All tests pass (including CI/CD)
- [ ] Documentation is updated
- [ ] Security implications considered
- [ ] Code reviewed and approved
- [ ] No breaking changes (or properly documented)
- [ ] Performance impact assessed
- [ ] Backward compatibility maintained

## 🙏 Recognition

Contributors will be:

- **Listed in CONTRIBUTORS.md** (if we create one)
- **Mentioned in release notes** for significant contributions
- **Credited in commit history** and PR descriptions
- **Invited to be maintainers** for consistent valuable contributions

## 📞 Getting Help

If you need help:

1. **Check existing documentation** and examples
2. **Search closed issues** for similar problems
3. **Open a GitHub Discussion** for general questions
4. **Reach out on social media** @hasiptimurtas
5. **Email for sensitive matters** hasip@timurtas.com

---

Thank you for contributing to Solana Watchtower! Your efforts help make Solana monitoring more accessible and reliable for everyone.

**Happy coding! 🚀** 