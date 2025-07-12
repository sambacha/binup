# GUP Architecture Guide

This document provides a detailed overview of GUP's internal architecture for developers and contributors.

## High-Level Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User Command  │    │   Project Tool  │    │   GUP Daemon    │
│                 │    │   (symlinked)   │    │   (future)      │
│ gup add node    │    │ node --version  │    │ Background      │
│ gup list        │    │ npm install     │    │ Updates         │
│ gup status      │    │ deno run        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                        GUP Core                                 │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   CLI Parser    │  │   Multiplexer   │  │  Configuration  │ │
│  │                 │  │                 │  │                 │ │
│  │ Command routing │  │ Version routing │  │ Global config   │ │
│  │ Argument        │  │ +version syntax │  │ Project state   │ │
│  │ validation      │  │ Default routing │  │ Settings        │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Operations    │  │    Metadata     │  │   Installation  │ │
│  │                 │  │                 │  │                 │ │
│  │ Project mgmt    │  │ Fetch metadata  │  │ Download        │ │
│  │ Version mgmt    │  │ Version resolve │  │ Extract         │ │
│  │ Garbage collect │  │ Channel resolve │  │ Symlink         │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  Error Handling │  │   File System   │  │   Networking    │ │
│  │                 │  │                 │  │                 │ │
│  │ User-friendly   │  │ Atomic writes   │  │ HTTP downloads  │ │
│  │ messages        │  │ Path resolution │  │ Checksum verify │ │
│  │ Recovery hints  │  │ Symlink mgmt    │  │ Retry logic     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Module Organization

### Core Modules

#### CLI Layer (`src/cli.rs`, `src/bin/gup.rs`)
- **Purpose**: Command-line interface definition and parsing
- **Key Components**:
  - `GupCli`: Main command enum using `clap`
  - Subcommand definitions (`ProjectSubCmd`, `SelfSubCmd`, etc.)
  - Argument validation and help text
- **Dependencies**: `clap`, `anyhow`

#### Command Implementations (`src/command_*.rs`)
Each command has its own module:
- `command_add.rs`: Install versions and channels
- `command_project.rs`: Project registration and management
- `command_list.rs`: List available versions and channels
- `command_status.rs`: Show installation status
- `command_remove.rs`: Remove versions
- `command_default.rs`: Set default versions
- `command_gc.rs`: Garbage collection
- `command_update.rs`: Update to latest versions

#### Operations Layer (`src/operations_*.rs`)
Core business logic:
- `operations_metadata.rs`: Metadata fetching and version resolution
- `operations_install.rs`: Installation workflow
- `operations_download.rs`: HTTP downloads with verification
- `operations_symlink.rs`: Symlink management
- `operations_gc.rs`: Cleanup operations

#### Configuration Management
- `global_config_manager.rs`: Global configuration (managed projects)
- `project_state_manager.rs`: Per-project state (installed versions)
- `state_config.rs`: Configuration data structures
- `global_paths.rs`: Path resolution and directory management

#### Data Models
- `project_metadata_m.rs`: Project metadata JSON structures
- `state_config.rs`: Internal state structures

#### Utilities
- `error_handling.rs`: User-friendly error messages
- `utils.rs`: Cross-platform utilities
- `multiplexer.rs`: Version routing for symlinked commands

### Data Flow Patterns

#### Command Execution Flow
```rust
// 1. CLI parsing (main.rs)
let cli = GupCli::parse();

// 2. Path resolution
let paths = get_gup_paths()?;

// 3. Command dispatch
match cli {
    GupCli::Add { project, version } => {
        run_command_add(&project, &version, &paths)?;
    }
    // ... other commands
}

// 4. Command implementation
pub fn run_command_add(
    project_name: &str,
    version_or_channel: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    // Load configuration
    let global_config = load_global_config(paths)?;
    let mut project_state = load_project_local_state(project_name, paths)?;
    
    // Resolve version
    let concrete_version = resolve_channel_to_version(
        version_or_channel,
        &metadata,
    )?;
    
    // Install version
    install_version(project_name, &concrete_version, &metadata, &mut project_state, paths)?;
    
    // Update state
    save_project_local_state(project_name, &project_state, paths)?;
    
    Ok(())
}
```

#### Version Resolution Flow
```rust
// 1. Load project metadata
let metadata = sync_project_metadata(project_name, &source_url, paths)?;

// 2. Try channel resolution first
if let Some(channel) = metadata.channels.get(version_or_channel) {
    match channel.resolution_strategy.as_str() {
        "latest_semver" => {
            // Find latest version matching prefix
            let versions = filter_versions_by_prefix(&metadata.available_versions, &channel.version_prefix);
            let latest = find_latest_semver(versions)?;
            return Ok(latest);
        }
        "exact_match" => {
            // Use exact version from prefix
            return Ok(channel.version_prefix.clone());
        }
    }
}

// 3. Try direct version lookup
if metadata.available_versions.contains_key(version_or_channel) {
    return Ok(version_or_channel.to_string());
}

// 4. Resolution failed
Err(GupError::version_not_found(project_name, version_or_channel, &available_versions))
```

#### Installation Flow
```rust
// 1. Determine current platform
let platform_id = get_current_platform_id(&metadata)?;

// 2. Get artifact for platform and version
let artifact = metadata
    .available_versions[version]
    .artifacts
    .get(&platform_id)
    .ok_or_else(|| anyhow!("No artifact for platform {}", platform_id))?;

// 3. Download artifact
let download_url = format!("{}{}", metadata.base_download_url, artifact.url_path_suffix);
let downloaded_file = download_file(&download_url, &artifact.sha256)?;

// 4. Extract to installation directory
let install_dir = paths.installation_dir(project_name, version);
extract_archive(&downloaded_file, &install_dir, &install_config)?;

// 5. Update symlinks if this is the default version
if is_default_version {
    update_default_symlinks_for_project(project_name, paths)?;
}
```

## Configuration System

### Directory Structure
```
~/.gup/                           # GUP_DEPOT_PATH
├── config.json                  # Global configuration
├── projects/                    # Per-project state
│   ├── node.json               #   Project state files
│   ├── deno.json
│   └── rust.json
├── installations/               # Installed versions
│   ├── node/
│   │   ├── 18.19.0/            #   Version directories
│   │   ├── 20.10.0/
│   │   └── 21.5.0/
│   ├── deno/
│   │   └── 1.40.0/
│   └── rust/
│       ├── 1.75.0/
│       └── 1.76.0/
├── bin/                         # Symlinks (in PATH)
│   ├── node -> gup             #   Command symlinks
│   ├── npm -> gup
│   ├── deno -> gup
│   ├── cargo -> gup
│   └── rustc -> gup
└── cache/                       # Download cache (future)
    ├── metadata/
    └── artifacts/
```

### Configuration Files

#### Global Configuration (`config.json`)
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GupGlobalConfig {
    pub managed_projects: HashMap<String, ManagedProjectInfo>,
    pub self_update_channel: Option<String>,
    pub background_self_update_interval_minutes: Option<u32>,
    pub modify_path: Option<bool>,
    pub symlinks_enabled: Option<bool>,
    pub default_channel: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagedProjectInfo {
    pub unique_name: String,
    pub display_name: String,
    pub source_of_truth_url: String,
    pub last_metadata_sync: Option<DateTime<Utc>>,
    pub preferred_default: Option<String>,
}
```

#### Project State (`projects/{project}.json`)
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProjectLocalState {
    pub installed_versions: HashMap<String, VersionInstallInfo>,
    pub active_channels: HashMap<String, ActiveChannelInfo>,
    pub default_version_or_channel_name: Option<String>,
    pub directory_overrides: HashMap<PathBuf, String>,
    pub symlinks: HashMap<String, SymlinkInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInstallInfo {
    pub version: String,
    pub install_timestamp: DateTime<Utc>,
    pub install_source: String,
    pub executables: Vec<String>,
}
```

## Error Handling Strategy

### Error Types Hierarchy
```rust
// User-facing errors with helpful suggestions
pub struct GupError;

impl GupError {
    pub fn invalid_url(url: &str, context: &str) -> anyhow::Error;
    pub fn network_error(url: &str, original_error: &anyhow::Error) -> anyhow::Error;
    pub fn project_not_found(project_name: &str) -> anyhow::Error;
    pub fn version_not_found(project_name: &str, version: &str, available: &[String]) -> anyhow::Error;
    pub fn invalid_metadata(url: &str, json_error: &serde_json::Error) -> anyhow::Error;
    pub fn http_error(url: &str, status_code: u16) -> anyhow::Error;
}
```

### Error Handling Patterns
```rust
// Convert system errors to user-friendly errors
let metadata_url = url::Url::parse(registration_source)
    .map_err(|_| GupError::invalid_url(registration_source, "project registration"))?;

// Provide context for operations
let response = reqwest::blocking::get(metadata_url.clone())
    .map_err(|e| GupError::network_error(registration_source, &e.into()))?;

// Chain errors with additional context
let project_info = global_config
    .managed_projects
    .get(project_name)
    .ok_or_else(|| GupError::project_not_found(project_name))?;
```

## Concurrency and Safety

### Thread Safety
- **Current**: Single-threaded CLI operations
- **Future**: Background daemon for updates and maintenance
- **Considerations**: 
  - Atomic file operations for configuration updates
  - File locking for concurrent access protection
  - Async/await for network operations

### File System Safety
```rust
// Atomic configuration updates
pub fn save_global_config(config: &GupGlobalConfig, paths: &GupGlobalPaths) -> Result<()> {
    // Write to temporary file first
    let temp_path = config_path.with_extension("json.tmp");
    fs::write(&temp_path, content)?;
    
    // Atomic rename
    fs::rename(&temp_path, &config_path)?;
    
    Ok(())
}

// Safe symlink updates
pub fn create_executable_symlink(
    executable_name: &str,
    gup_executable_path: &Path,
    paths: &GupGlobalPaths,
) -> Result<()> {
    // Remove existing symlink if present
    let _prev_target = _remove_symlink(&symlink_path)?;
    
    // Create new symlink
    unix_fs::symlink(&gup_executable_path, &symlink_path)?;
    
    Ok(())
}
```

## Testing Architecture

### Test Organization
```
tests/
├── integration_tests/           # End-to-end CLI tests
│   ├── gup_integration_test.rs #   Main integration tests
│   ├── command_add.rs          #   Command-specific tests
│   └── command_list_test.rs
├── config_management_test.rs    # Configuration system tests
├── project_state_test.rs        # Project state tests
└── examples/                    # Test data
    ├── nodejs-metadata.json     #   Example metadata files
    ├── deno-metadata.json
    └── simple-test-metadata.json
```

### Test Utilities
```rust
// Test environment setup
struct TestEnvironment {
    _temp_dir: TempDir,
    paths: GupGlobalPaths,
}

impl TestEnvironment {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        env::set_var("GUP_DEPOT_PATH", temp_dir.path());
        let paths = get_gup_paths()?;
        Ok(TestEnvironment { _temp_dir: temp_dir, paths })
    }
}

// Property-based testing patterns
#[test]
fn test_version_resolution_properties() {
    // Test that version resolution is deterministic
    // Test that channels always resolve to valid versions
    // Test that invalid versions are properly rejected
}
```

## Platform Abstraction

### Cross-Platform Compatibility
```rust
// Platform-specific implementations
#[cfg(unix)]
pub fn create_executable_symlink(/* ... */) -> Result<()> {
    unix_fs::symlink(gup_executable_path, symlink_path)?;
    Ok(())
}

#[cfg(windows)]
pub fn create_executable_symlink(/* ... */) -> Result<()> {
    // Windows-specific shim creation (future implementation)
    todo!("Windows symlink/shim creation not yet implemented")
}

// Path handling
pub fn get_gup_paths() -> Result<GupGlobalPaths> {
    let depot_path = if let Ok(path) = env::var("GUP_DEPOT_PATH") {
        PathBuf::from(path)
    } else {
        #[cfg(windows)]
        let default_path = dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not determine data directory"))?
            .join("gup");
            
        #[cfg(not(windows))]
        let default_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".gup");
            
        default_path
    };
    
    Ok(GupGlobalPaths::new(depot_path))
}
```

## Performance Considerations

### Optimization Strategies
- **Lazy Loading**: Load configuration and metadata only when needed
- **Caching**: Cache metadata and downloads (future feature)
- **Parallel Operations**: Concurrent downloads for multiple versions
- **Minimal I/O**: Reduce file system operations in hot paths

### Memory Management
```rust
// Stream large downloads instead of loading into memory
pub fn download_file(url: &str, expected_sha256: Option<&str>) -> Result<PathBuf> {
    let mut response = reqwest::blocking::get(url)?;
    let temp_file = tempfile::NamedTempFile::new()?;
    
    // Stream to file with hash calculation
    let mut hasher = Sha256::new();
    std::io::copy(&mut response, &mut BufWriter::new(&temp_file))?;
    
    // Verify checksum without loading file into memory
    if let Some(expected) = expected_sha256 {
        verify_checksum(&temp_file.path(), expected)?;
    }
    
    Ok(temp_file.into_temp_path().persist()?)
}
```

## Future Architecture Considerations

### Planned Features
- **Background Daemon**: Automatic updates and maintenance
- **Plugin System**: Extensible project support
- **Distributed Caching**: Shared artifact cache
- **Web UI**: Browser-based management interface

### Scalability Improvements
- **Database Backend**: Replace JSON files with SQLite for better performance
- **Content Delivery Network**: Distribute metadata and artifacts globally
- **Incremental Updates**: Delta downloads for large artifacts
- **Compression**: Compress metadata and state files

### Security Enhancements
- **Signature Verification**: Cryptographic signatures for metadata and artifacts
- **Sandboxed Installation**: Isolated installation environments
- **Audit Logging**: Track all operations for security analysis
- **Permission Management**: Fine-grained access controls