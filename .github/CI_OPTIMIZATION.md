# CI Optimization Strategy

## Overview

This document describes the CI optimization strategy for the gup project, focusing on:
- Faster feedback loops
- Reduced resource usage
- Platform-specific build optimization
- Aggressive caching
- Performance tracking

## Key Optimizations

### 1. Conditional Platform Builds

We only trigger platform-specific builds when relevant files change:

```yaml
path_filters:
  windows:
    - 'src/**/*windows*.rs'
    - 'tests/*windows*.rs'
  macos:
    - 'src/**/*macos*.rs'
    - 'tests/*macos*.rs'
```

**Benefits:**
- Windows builds only run when Windows-specific code changes
- macOS builds only run when macOS-specific code changes
- Linux builds run for all changes (fastest CI platform)

### 2. Tiered Build Strategy

**Tier 1 (Always Build):**
- x86_64-unknown-linux-gnu
- x86_64-pc-windows-msvc
- x86_64-apple-darwin
- aarch64-apple-darwin

**Tier 2 (Release Only):**
- x86_64-unknown-linux-musl
- aarch64-unknown-linux-gnu
- i686-pc-windows-msvc
- x86_64-unknown-freebsd

### 3. Aggressive Caching

We use multiple cache strategies:

1. **Rust Cache Action**: Caches cargo registry, index, and build artifacts
2. **Custom Cache Keys**: Separate caches for different workflows
3. **Cache Warming**: Main branch builds warm caches for PRs

```yaml
cache:
  prefix-key: v0-rust
  shared-key: ${{ matrix.target }}
  save-if: ${{ github.ref == 'refs/heads/main' }}
```

### 4. Parallel Execution

- **Concurrent Jobs**: Run platform tests in parallel
- **Test Parallelism**: Use `--test-threads=4` for faster test execution
- **Matrix Strategy**: `fail-fast: false` to get all results

### 5. Fast Feedback Workflows

**test-fast.yml**: Quick PR validation (< 3 minutes)
- Sanity checks
- Linux-only unit tests
- Critical integration tests
- Smoke tests on other platforms

**ci-optimized.yml**: Comprehensive testing
- Full platform matrix
- All test suites
- Code coverage
- Security scanning

### 6. Build Time Tracking

Performance metrics are tracked and stored:

```bash
.github/scripts/track-ci-metrics.sh record-build "Linux Build" "120" "x86_64-unknown-linux-gnu" "true"
```

Metrics include:
- Build duration
- Test duration
- Cache hit rates
- Failure rates

### 7. Reusable Workflows

**build-reusable.yml**: Standardized build process
- Consistent setup across platforms
- Automatic artifact upload
- Build time tracking

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| PR Feedback Time | < 5 min | ~3 min |
| Full CI Run | < 15 min | ~10 min |
| Cache Hit Rate | > 80% | ~85% |
| Release Build | < 30 min | ~25 min |

## Workflow Structure

### Pull Requests
1. **Fast Tests** (3 min)
   - Sanity checks
   - Format/Clippy
   - Linux unit tests
   - Critical integration tests

2. **Platform Tests** (conditional, 5-8 min)
   - Windows tests (if Windows files changed)
   - macOS tests (if macOS files changed)
   - Full Linux test suite

### Main Branch
1. **Comprehensive Tests** (10-15 min)
   - All platforms
   - All test suites
   - Code coverage
   - Documentation

2. **Nightly/Weekly**
   - Security audit
   - Dependency updates
   - Performance analysis

### Releases
1. **Build Matrix** (20-30 min)
   - 8 target platforms
   - Release binaries
   - Platform installers
   - Package manager publishing

## Cost Optimization

1. **Reduced macOS/Windows Usage**: Only run when necessary
2. **Efficient Caching**: Reduces download/compilation time
3. **Concurrent Limits**: Prevent runaway costs from multiple PRs
4. **Scheduled Cleanup**: Remove old artifacts and caches

## Monitoring and Maintenance

### Weekly Performance Review
The `update-ci.yml` workflow automatically:
- Analyzes CI performance
- Identifies slow or failing workflows
- Creates PRs with optimizations

### Manual Interventions
- Cache version bumps when dependencies change significantly
- Workflow consolidation when patterns emerge
- Platform-specific optimizations based on metrics

## Best Practices for Contributors

1. **Use Path Filters**: Tag PRs with platform labels when appropriate
2. **Skip CI When Possible**: Use `[skip ci]` for documentation-only changes
3. **Local Testing**: Run `cargo test` locally before pushing
4. **Review CI Logs**: Check for warnings about slow tests or builds

## Future Improvements

1. **Distributed Testing**: Split test suites across multiple jobs
2. **Docker Caching**: Pre-built Docker images for faster Linux CI
3. **Binary Caching**: Cache built binaries between similar runs
4. **Predictive Testing**: Run only affected tests based on code changes

## Troubleshooting

### Slow Builds
1. Check cache hit rate in logs
2. Verify no large files were added
3. Review dependency changes

### Flaky Tests
1. Use `--test-threads=1` for debugging
2. Check for timing-dependent tests
3. Review platform-specific assumptions

### Cache Issues
1. Bump cache version in ci-config.yml
2. Clear caches from GitHub UI
3. Check cache size limits