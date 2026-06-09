# GitHub Actions Workflows

This repository includes comprehensive GitHub Actions workflows for continuous integration, testing, and automated releases.

## 🚀 Workflows Overview

### 1. **CI Workflow** (`.github/workflows/ci.yml`)
**Triggers**: Push to `main`/`develop`, Pull Requests
**Purpose**: Continuous integration with testing and building

**Features**:
- ✅ **Multi-Rust Testing**: Tests on stable, 1.89.0, and nightly Rust versions
- 🏗️ **Cross-Platform Builds**: Builds on Ubuntu, Windows, and macOS
- 🔒 **Security Audits**: Runs `cargo audit` for vulnerability scanning
- 📊 **Code Coverage**: Generates coverage reports with `cargo-tarpaulin`
- 🔍 **Dependency Review**: Automatically reviews dependency changes in PRs

**Jobs**:
- `test`: Runs tests on multiple Rust versions with coverage
- `build`: Cross-platform builds (Linux, Windows, macOS)
- `security`: Security vulnerability scanning
- `dependency-review`: Dependency change analysis

### 2. **Release Workflow** (`.github/workflows/release.yml`)
**Triggers**: Push of version tags (e.g., `v1.0.0`)
**Purpose**: Automated release creation and binary distribution

**Features**:
- 🏷️ **Automatic Releases**: Creates GitHub releases from version tags
- 📦 **Cross-Platform Binaries**: Builds for Linux, Windows, and macOS
- 📝 **Auto-Generated Changelog**: Creates changelog from git commits
- 🔧 **Binary Optimization**: Strips debug symbols for smaller binaries
- 📋 **Release Notes**: Comprehensive release documentation

**How to Release**:
```bash
# 1. Update version in Cargo.toml
version = "1.0.0"

# 2. Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to 1.0.0"
git tag v1.0.0

# 3. Push tag to trigger release
git push origin v1.0.0
```

**Release Assets Created**:
- `petrify-x86_64-unknown-linux-gnu.tar.gz` - Linux binary
- `petrify-x86_64-pc-windows-msvc.zip` - Windows binary
- `petrify-x86_64-apple-darwin.tar.gz` - macOS binary

### 3. **Dependencies Workflow** (`.github/workflows/dependencies.yml`)
**Triggers**: Weekly schedule (Mondays 2 AM UTC), Manual dispatch
**Purpose**: Automated dependency management and security updates

**Features**:
- 🔒 **Security Updates**: Automatically updates dependencies with security fixes
- 📦 **Dependency Updates**: Updates all dependencies to latest versions
- 🔄 **Major Version Updates**: Option to update to major versions
- 📋 **Auto-PR Creation**: Creates pull requests for dependency updates
- ⏰ **Scheduled Runs**: Weekly automated security scanning

**Manual Triggers**:
- **Security**: Update only security-related dependencies
- **All**: Update all dependencies to latest compatible versions
- **Major**: Update to major versions (may include breaking changes)

### 4. **Code Quality Workflow** (`.github/workflows/code-quality.yml`)
**Triggers**: Push to `main`/`develop`, Pull Requests, Manual dispatch
**Purpose**: Code quality assurance and documentation

**Features**:
- 🎨 **Code Formatting**: Ensures consistent code style with `rustfmt`
- 🔍 **Linting**: Catches code issues with `clippy`
- 📚 **Documentation**: Generates and checks documentation
- 📊 **Benchmarks**: Performance benchmarking with `criterion`
- 🔒 **License Compliance**: Checks for license compatibility
- 📈 **Dependency Graphs**: Visual dependency analysis

## 🛠️ Setup and Configuration

### Prerequisites
- GitHub repository with Actions enabled
- Rust project with `Cargo.toml` and `Cargo.lock`
- Proper versioning in `Cargo.toml`

### Required Secrets
The workflows use the default `GITHUB_TOKEN` secret, which is automatically provided by GitHub.

### Optional Integrations
- **Codecov**: For code coverage reporting
- **Dependabot**: For additional dependency management

## 📋 Workflow Usage

### For Developers
1. **Push to main/develop**: Triggers CI and code quality checks
2. **Create PR**: Triggers dependency review and quality checks
3. **Tag releases**: Automatically creates releases and distributes binaries

### For Maintainers
1. **Monitor workflows**: Check Actions tab for workflow status
2. **Review dependency updates**: Approve or modify auto-generated PRs
3. **Manage releases**: Use version tags to trigger releases
4. **Quality assurance**: Monitor code quality metrics and coverage

### For Contributors
1. **Follow contribution guidelines**: Ensure code passes all quality checks
2. **Test locally**: Run `cargo test`, `cargo fmt`, `cargo clippy`
3. **Update dependencies**: Use the dependency workflow for updates

## 🔧 Customization

### Modifying Workflows
- Edit `.github/workflows/*.yml` files
- Adjust triggers, jobs, and steps as needed
- Add new workflows for specific requirements

### Adding New Platforms
- Modify the matrix in build jobs
- Add platform-specific build steps
- Update release asset creation

### Customizing Release Process
- Modify release notes template
- Add additional release assets
- Customize changelog generation

## 📊 Monitoring and Metrics

### Workflow Status
- **Green**: All checks passed
- **Yellow**: Some non-critical issues
- **Red**: Critical failures requiring attention

### Key Metrics
- **Test Coverage**: Aim for >80% coverage
- **Build Success Rate**: Should be >95%
- **Security Issues**: Zero vulnerabilities
- **Dependency Health**: Regular updates and security patches

### Artifacts
- **Coverage Reports**: HTML coverage reports
- **Benchmark Results**: Performance metrics
- **Dependency Graphs**: Visual dependency analysis
- **Build Logs**: Detailed build and test logs

## 🚨 Troubleshooting

### Common Issues
1. **Workflow failures**: Check logs for specific error messages
2. **Build errors**: Verify Rust version compatibility
3. **Test failures**: Run tests locally to reproduce issues
4. **Release failures**: Check tag format and permissions

### Debugging Steps
1. **Check workflow logs**: Detailed error information
2. **Reproduce locally**: Run commands from failed steps
3. **Verify dependencies**: Check `Cargo.lock` and dependencies
4. **Check permissions**: Ensure proper GitHub token permissions

## 📚 Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust GitHub Actions](https://github.com/actions-rs)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [Rust Toolchain Management](https://rust-lang.github.io/rustup/)

## 🤝 Contributing

To improve these workflows:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test locally
5. Submit a pull request

## 📄 License

These workflows are part of the petrify project and follow the same license terms. 