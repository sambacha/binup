# GUP Project Metadata Format Specification

Version: 1.0.0

This document defines the JSON metadata format that GUP uses to manage software projects. Project metadata files describe how to download, install, and manage different versions of a tool or application.

## Overview

A GUP metadata file is a JSON document that contains:
- Project identification and description
- Platform definitions and compatibility
- Available versions and their artifacts
- Installation configuration
- Channel definitions for version resolution

## Schema

### Root Object

```json
{
  "project_name": "string",
  "display_name": "string",
  "description": "string", 
  "homepage_url": "string",
  "metadata_format_version": "string",
  "base_download_url": "string",
  "platforms": { "PlatformDetail": {} },
  "default_install_config": "InstallConfig",
  "executables": ["ExecutableDetail"],
  "default_executable_name": "string",
  "available_versions": { "VersionDetail": {} },
  "channels": { "ChannelDetail": {} }
}
```

#### Required Fields

- **`project_name`** (string): Unique identifier for the project. Used for commands and file paths. Must be a valid filename.
- **`metadata_format_version`** (string): Version of this metadata format. Currently "1.0.0".
- **`base_download_url`** (string): Base URL for downloading artifacts. Artifact `url_path_suffix` values are appended to this.
- **`platforms`** (object): Map of platform identifiers to platform details.
- **`default_install_config`** (object): Default installation configuration.
- **`executables`** (array): List of executable files the project provides.
- **`available_versions`** (object): Map of version strings to version details.

#### Optional Fields

- **`display_name`** (string): Human-readable project name. Defaults to `project_name`.
- **`description`** (string): Project description.
- **`homepage_url`** (string): Project homepage URL.
- **`default_executable_name`** (string): Name of the primary executable. Used for symlink naming.
- **`channels`** (object): Map of channel names to channel details. Defaults to empty.

### PlatformDetail

Describes a target platform that the project supports.

```json
{
  "os": "string",
  "arch": "string", 
  "target_triple_pattern": "string"
}
```

#### Fields

- **`os`** (string): Operating system name (`linux`, `windows`, `macos`, `freebsd`).
- **`arch`** (string): Architecture (`x86_64`, `aarch64`, `i686`).
- **`target_triple_pattern`** (string): Regex pattern to match against the system's target triple.

#### Common Platform IDs

- `linux-x64`: Linux x86_64
- `linux-arm64`: Linux ARM64
- `macos-x64`: macOS x86_64 (Intel)
- `macos-arm64`: macOS ARM64 (Apple Silicon)
- `windows-x64`: Windows x86_64
- `windows-x86`: Windows 32-bit

### InstallConfig

Describes how to install and extract project artifacts.

```json
{
  "archive_format": "string",
  "strip_components": "number",
  "bin_subdir": "string",
  "post_install_hook": "string"
}
```

#### Fields

- **`archive_format`** (string): Archive format (`tar.gz`, `zip`, `binary`). Defaults to `tar.gz`.
- **`strip_components`** (number): Number of directory levels to strip when extracting. Defaults to 0.
- **`bin_subdir`** (string): Subdirectory containing executables relative to extraction root. Defaults to `""`.
- **`post_install_hook`** (string): Shell command to run after installation. Optional.

#### Archive Formats

- **`tar.gz`**: Gzipped tar archive (most common for Unix)
- **`zip`**: ZIP archive (common for Windows)
- **`binary`**: Single executable file (no extraction needed)

### ExecutableDetail

Describes an executable file provided by the project.

```json
{
  "name": "string",
  "path_in_bin_subdir": "string"
}
```

#### Fields

- **`name`** (string): Command name for the executable. Used for symlink creation.
- **`path_in_bin_subdir`** (string): Path to the executable relative to `bin_subdir`.

### VersionDetail

Describes a specific version of the project.

```json
{
  "release_date": "string",
  "artifacts": { "ArtifactDetail": {} },
  "install_config_override": "InstallConfig"
}
```

#### Fields

- **`release_date`** (string): ISO 8601 date when the version was released.
- **`artifacts`** (object): Map of platform IDs to artifact details.
- **`install_config_override`** (object): Installation config overrides for this version. Optional.

### ArtifactDetail

Describes a downloadable artifact for a specific version and platform.

```json
{
  "url_path_suffix": "string",
  "sha256": "string",
  "install_config_override": "InstallConfig"
}
```

#### Fields

- **`url_path_suffix`** (string): Path appended to `base_download_url` to get the download URL.
- **`sha256`** (string): SHA256 checksum for verification. Optional but recommended.
- **`install_config_override`** (object): Installation config overrides for this artifact. Optional.

### ChannelDetail

Describes a channel for version resolution.

```json
{
  "resolution_strategy": "string",
  "version_prefix": "string"
}
```

#### Fields

- **`resolution_strategy`** (string): How to resolve the channel to a version (`latest_semver`, `exact_match`).
- **`version_prefix`** (string): Version prefix filter. Required for `exact_match`, optional for `latest_semver`.

#### Resolution Strategies

- **`latest_semver`**: Find the latest version using semantic versioning. Optionally filter by `version_prefix`.
- **`exact_match`**: Use the exact version specified in `version_prefix`.

## Examples

### Simple Single-Binary Project

```json
{
  "project_name": "hello-world",
  "display_name": "Hello World Tool",
  "metadata_format_version": "1.0.0",
  "base_download_url": "https://github.com/example/hello-world/releases/download",
  "platforms": {
    "linux-x64": {
      "os": "linux",
      "arch": "x86_64",
      "target_triple_pattern": "x86_64.*linux.*"
    }
  },
  "default_install_config": {
    "archive_format": "binary"
  },
  "executables": [
    {
      "name": "hello",
      "path_in_bin_subdir": "hello"
    }
  ],
  "default_executable_name": "hello",
  "available_versions": {
    "1.0.0": {
      "release_date": "2024-01-01T00:00:00Z",
      "artifacts": {
        "linux-x64": {
          "url_path_suffix": "/v1.0.0/hello-linux-x64",
          "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }
      }
    }
  },
  "channels": {
    "stable": {
      "resolution_strategy": "latest_semver"
    }
  }
}
```

### Complex Multi-Platform Project

```json
{
  "project_name": "complex-tool",
  "display_name": "Complex Development Tool",
  "description": "A complex tool with multiple executables and platforms",
  "homepage_url": "https://complex-tool.dev",
  "metadata_format_version": "1.0.0",
  "base_download_url": "https://releases.complex-tool.dev",
  "platforms": {
    "linux-x64": {
      "os": "linux",
      "arch": "x86_64",
      "target_triple_pattern": "x86_64.*linux.*"
    },
    "linux-arm64": {
      "os": "linux", 
      "arch": "aarch64",
      "target_triple_pattern": "aarch64.*linux.*"
    },
    "macos-x64": {
      "os": "macos",
      "arch": "x86_64", 
      "target_triple_pattern": "x86_64.*darwin.*"
    },
    "macos-arm64": {
      "os": "macos",
      "arch": "aarch64",
      "target_triple_pattern": "aarch64.*darwin.*"
    },
    "windows-x64": {
      "os": "windows",
      "arch": "x86_64",
      "target_triple_pattern": "x86_64.*windows.*"
    }
  },
  "default_install_config": {
    "archive_format": "tar.gz",
    "strip_components": 1,
    "bin_subdir": "bin"
  },
  "executables": [
    {
      "name": "complex-tool",
      "path_in_bin_subdir": "complex-tool"
    },
    {
      "name": "complex-helper", 
      "path_in_bin_subdir": "helper"
    }
  ],
  "default_executable_name": "complex-tool",
  "available_versions": {
    "1.0.0": {
      "release_date": "2024-01-01T00:00:00Z",
      "artifacts": {
        "linux-x64": {
          "url_path_suffix": "/1.0.0/complex-tool-1.0.0-linux-x64.tar.gz",
          "sha256": "abc123..."
        },
        "macos-x64": {
          "url_path_suffix": "/1.0.0/complex-tool-1.0.0-macos-x64.tar.gz",
          "sha256": "def456..."
        },
        "windows-x64": {
          "url_path_suffix": "/1.0.0/complex-tool-1.0.0-windows-x64.zip",
          "sha256": "ghi789...",
          "install_config_override": {
            "archive_format": "zip"
          }
        }
      }
    },
    "1.1.0": {
      "release_date": "2024-02-01T00:00:00Z",
      "artifacts": {
        "linux-x64": {
          "url_path_suffix": "/1.1.0/complex-tool-1.1.0-linux-x64.tar.gz",
          "sha256": "jkl012..."
        },
        "linux-arm64": {
          "url_path_suffix": "/1.1.0/complex-tool-1.1.0-linux-arm64.tar.gz", 
          "sha256": "mno345..."
        },
        "macos-x64": {
          "url_path_suffix": "/1.1.0/complex-tool-1.1.0-macos-x64.tar.gz",
          "sha256": "pqr678..."
        },
        "macos-arm64": {
          "url_path_suffix": "/1.1.0/complex-tool-1.1.0-macos-arm64.tar.gz",
          "sha256": "stu901..."
        },
        "windows-x64": {
          "url_path_suffix": "/1.1.0/complex-tool-1.1.0-windows-x64.zip",
          "sha256": "vwx234...",
          "install_config_override": {
            "archive_format": "zip"
          }
        }
      }
    },
    "2.0.0-beta1": {
      "release_date": "2024-03-01T00:00:00Z",
      "artifacts": {
        "linux-x64": {
          "url_path_suffix": "/2.0.0-beta1/complex-tool-2.0.0-beta1-linux-x64.tar.gz",
          "sha256": "yza567..."
        }
      }
    }
  },
  "channels": {
    "stable": {
      "resolution_strategy": "latest_semver"
    },
    "beta": {
      "version_prefix": "2.0.0-beta",
      "resolution_strategy": "latest_semver"
    },
    "v1": {
      "version_prefix": "1.",
      "resolution_strategy": "latest_semver" 
    },
    "lts": {
      "version_prefix": "1.1.0",
      "resolution_strategy": "exact_match"
    }
  }
}
```

## Configuration Precedence

Installation configuration is resolved with the following precedence (later overrides earlier):

1. **Default**: Built-in defaults (`archive_format: "tar.gz"`, `strip_components: 0`)
2. **Project Default**: `default_install_config` 
3. **Version Override**: `available_versions[version].install_config_override`
4. **Artifact Override**: `available_versions[version].artifacts[platform].install_config_override`

## Validation Rules

### Project Names

- Must be valid filenames on all target platforms
- Should use lowercase letters, numbers, and hyphens
- Must not start or end with hyphens
- Should be unique across the GUP ecosystem

### Version Strings

- Must be valid semantic version strings for `latest_semver` resolution
- Can be any string for `exact_match` resolution
- Should follow semantic versioning best practices

### URLs

- `base_download_url` must be a valid HTTP/HTTPS URL
- `homepage_url` must be a valid HTTP/HTTPS URL
- Combined `base_download_url + url_path_suffix` must be valid URLs

### Platform Compatibility

- `target_triple_pattern` must be valid regex
- Platform IDs should be consistent within a project
- All versions should support the same set of platforms (where possible)

## Migration Guide

### From Other Version Managers

When migrating metadata from other version management systems:

1. **Node.js (nvm)**: Map `node` versions to GUP format with platform-specific artifacts
2. **Ruby (rbenv)**: Convert Ruby versions with appropriate compile configurations  
3. **Python (pyenv)**: Map Python distributions to platform-specific downloads
4. **Java (SDKMAN)**: Convert JDK distributions to GUP metadata format

### Versioning Strategy

- Use semantic versioning for `available_versions` keys
- Create meaningful channel names (`stable`, `lts`, `beta`)
- Maintain backward compatibility when updating metadata
- Consider deprecation strategy for old versions

## Best Practices

### Metadata Maintenance

- ✅ Keep metadata files under version control
- ✅ Automate metadata updates from CI/CD
- ✅ Validate metadata with JSON schema tools
- ✅ Use consistent naming conventions
- ✅ Include SHA256 checksums for security
- ✅ Test metadata changes before deployment

### Performance Optimization

- ✅ Host metadata files on CDN for global availability
- ✅ Implement caching headers for metadata files
- ✅ Keep metadata files reasonably sized (< 1MB recommended)
- ✅ Consider splitting large metadata files by major version

### Security Considerations

- ✅ Always include SHA256 checksums
- ✅ Use HTTPS for all URLs
- ✅ Validate artifact integrity during installation
- ✅ Consider signing metadata files for additional security
- ✅ Regular security audits of hosted artifacts