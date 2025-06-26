# GUP Troubleshooting Guide

This guide helps resolve common issues when using GUP.

## Installation Issues

### Command Not Found After Installation

**Problem**: `gup: command not found` after installation.

**Solutions**:

1. **Check if GUP is in PATH**:
   ```bash
   # Check current PATH
   echo $PATH
   
   # Look for GUP directory
   ls -la ~/.gup/bin/
   ```

2. **Add GUP to PATH**:
   ```bash
   # For bash/zsh
   echo 'export PATH="$HOME/.gup/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   
   # For fish
   echo 'set -gx PATH $HOME/.gup/bin $PATH' >> ~/.config/fish/config.fish
   
   # For PowerShell (Windows)
   $env:PATH += ";$env:APPDATA\gup\bin"
   ```

3. **Verify installation**:
   ```bash
   which gup
   gup --version
   ```

### Permission Denied Errors

**Problem**: Permission errors during installation or operation.

**Solutions**:

1. **Check directory permissions**:
   ```bash
   ls -la ~/.gup/
   ls -la ~/.gup/bin/
   ```

2. **Fix permissions**:
   ```bash
   chmod 755 ~/.gup/bin/gup
   chmod -R u+w ~/.gup/
   ```

3. **Check disk space**:
   ```bash
   df -h ~/.gup/
   ```

### Windows-Specific Issues

**Problem**: GUP not working properly on Windows.

**Solutions**:

1. **Check PowerShell execution policy**:
   ```powershell
   Get-ExecutionPolicy
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

2. **Use Windows Subsystem for Linux (WSL)**:
   ```bash
   # Install in WSL for better compatibility
   curl -fsSL https://install.gup.dev | sh
   ```

3. **Check antivirus software**:
   - Add GUP directory to antivirus exclusions
   - Temporarily disable real-time protection during installation

## Project Management Issues

### Project Not Found

**Problem**: `Error: Project not found` when trying to add versions.

**Solutions**:

1. **Check registered projects**:
   ```bash
   gup project list
   ```

2. **Register the project first**:
   ```bash
   gup project add https://example.com/project-metadata.json
   ```

3. **Verify project name**:
   ```bash
   # Check exact project name (case-sensitive)
   gup project list
   gup add correct-project-name 1.0.0
   ```

### Invalid Project Metadata

**Problem**: `Error: Invalid project metadata` when adding projects.

**Solutions**:

1. **Check metadata URL**:
   ```bash
   # Test URL directly
   curl -f https://example.com/metadata.json
   ```

2. **Validate JSON format**:
   ```bash
   # Use jq to validate JSON
   curl -s https://example.com/metadata.json | jq .
   ```

3. **Check required fields**:
   - Ensure `project_name`, `metadata_format_version`, and `platforms` are present
   - See [Metadata Format Guide](METADATA_FORMAT.md) for complete specification

### Network Connectivity Issues

**Problem**: Failed to download metadata or artifacts.

**Solutions**:

1. **Check internet connectivity**:
   ```bash
   ping google.com
   curl -I https://httpbin.org/status/200
   ```

2. **Check proxy settings**:
   ```bash
   # Set proxy if needed
   export HTTP_PROXY=http://proxy.company.com:8080
   export HTTPS_PROXY=http://proxy.company.com:8080
   ```

3. **Try alternative DNS**:
   ```bash
   # Test with different DNS
   nslookup example.com 8.8.8.8
   ```

4. **Check firewall**:
   - Ensure GUP can access HTTPS (port 443)
   - Whitelist GUP in corporate firewalls

## Version Management Issues

### Version Not Found

**Problem**: `Error: Version not found` when installing specific versions.

**Solutions**:

1. **List available versions**:
   ```bash
   gup list project-name
   ```

2. **Check version format**:
   ```bash
   # Use exact version strings
   gup add project-name 1.2.3  # not v1.2.3
   ```

3. **Try channel instead**:
   ```bash
   gup add project-name stable
   gup add project-name latest
   ```

### Installation Failures

**Problem**: Version installation fails with download or extraction errors.

**Solutions**:

1. **Check available disk space**:
   ```bash
   df -h ~/.gup/
   ```

2. **Verify checksums**:
   ```bash
   # GUP automatically verifies SHA256 checksums
   # If this fails, the artifact may be corrupted
   gup add project-name version --force  # retry download
   ```

3. **Check platform compatibility**:
   ```bash
   # Ensure your platform is supported
   uname -m  # check architecture
   uname -s  # check OS
   ```

4. **Manual cleanup and retry**:
   ```bash
   # Remove partial installation
   rm -rf ~/.gup/installations/project-name/version
   gup add project-name version
   ```

### Symlink Issues

**Problem**: Installed tools not available in PATH or wrong version being used.

**Solutions**:

1. **Check symlinks**:
   ```bash
   ls -la ~/.gup/bin/
   readlink ~/.gup/bin/tool-name
   ```

2. **Recreate symlinks**:
   ```bash
   # Set default version to recreate symlinks
   gup default project-name version
   ```

3. **Check PATH precedence**:
   ```bash
   # Ensure GUP bin directory comes first
   which tool-name
   type tool-name
   ```

4. **Check for conflicts**:
   ```bash
   # Look for other installations
   which -a tool-name
   ```

## Performance Issues

### Slow Downloads

**Problem**: Downloads are very slow or timing out.

**Solutions**:

1. **Check internet speed**:
   ```bash
   # Test download speed
   curl -o /dev/null -s -w "Downloaded at %{speed_download} bytes/sec\n" \
     http://speedtest.tele2.net/10MB.zip
   ```

2. **Use different mirror**:
   ```bash
   # Some projects offer multiple download mirrors
   # Check project documentation for alternatives
   ```

3. **Increase timeout**:
   ```bash
   # Set environment variable for longer timeouts
   export GUP_DOWNLOAD_TIMEOUT=300  # 5 minutes
   ```

### High Disk Usage

**Problem**: GUP using too much disk space.

**Solutions**:

1. **Check disk usage**:
   ```bash
   du -sh ~/.gup/
   du -sh ~/.gup/installations/*
   ```

2. **Clean up old versions**:
   ```bash
   gup gc
   gup gc --prune-linked
   ```

3. **Remove unused projects**:
   ```bash
   gup project remove unused-project
   ```

4. **Configure automatic cleanup**:
   ```bash
   # Add to crontab for weekly cleanup
   echo "0 0 * * 0 gup gc" | crontab -
   ```

### Memory Issues

**Problem**: GUP consuming too much memory.

**Solutions**:

1. **Check for memory leaks**:
   ```bash
   # Monitor memory usage
   top -p $(pgrep gup)
   ```

2. **Restart GUP daemon** (if applicable):
   ```bash
   # Kill any running GUP processes
   pkill gup
   ```

3. **Reduce concurrent operations**:
   ```bash
   # Install versions one at a time instead of parallel
   gup add project1 version1
   gup add project2 version2
   ```

## Data Corruption Issues

### Corrupted Configuration

**Problem**: GUP configuration appears corrupted or invalid.

**Solutions**:

1. **Backup current config**:
   ```bash
   cp ~/.gup/config.json ~/.gup/config.json.backup
   ```

2. **Validate configuration**:
   ```bash
   cat ~/.gup/config.json | jq .
   ```

3. **Reset configuration**:
   ```bash
   # Remove corrupted config (will be recreated)
   rm ~/.gup/config.json
   gup status  # triggers config recreation
   ```

4. **Restore from backup**:
   ```bash
   cp ~/.gup/config.json.backup ~/.gup/config.json
   ```

### Corrupted Installations

**Problem**: Installed versions appear corrupted or incomplete.

**Solutions**:

1. **Verify installation integrity**:
   ```bash
   # Check if executables exist and are valid
   ls -la ~/.gup/installations/project/version/
   ```

2. **Reinstall specific version**:
   ```bash
   gup remove project version
   gup add project version
   ```

3. **Full project reinstall**:
   ```bash
   gup project remove project-name
   gup project add https://example.com/metadata.json
   gup add project-name stable
   ```

## Environment Issues

### Multiple GUP Installations

**Problem**: Conflicts between different GUP installations.

**Solutions**:

1. **Find all GUP installations**:
   ```bash
   which -a gup
   find / -name "gup" -type f 2>/dev/null
   ```

2. **Remove old installations**:
   ```bash
   # Remove system-wide installation
   sudo rm /usr/local/bin/gup
   
   # Remove user installation
   rm ~/.local/bin/gup
   ```

3. **Use explicit path**:
   ```bash
   # Use full path to specific GUP installation
   ~/.gup/bin/gup status
   ```

### Shell Configuration Conflicts

**Problem**: Shell configuration interfering with GUP.

**Solutions**:

1. **Check shell startup files**:
   ```bash
   # Look for PATH modifications
   grep -r "gup\|GUP" ~/.bashrc ~/.zshrc ~/.profile
   ```

2. **Test with clean environment**:
   ```bash
   # Start clean shell
   env -i bash --norc --noprofile
   export PATH="/usr/bin:/bin:$HOME/.gup/bin"
   gup status
   ```

3. **Check for aliases**:
   ```bash
   alias | grep gup
   type gup
   ```

## Diagnostic Information

### Collecting Debug Information

When reporting issues, include this diagnostic information:

```bash
# System information
uname -a
echo "Shell: $SHELL"
echo "PATH: $PATH"

# GUP information
gup --version
echo "GUP_DEPOT_PATH: ${GUP_DEPOT_PATH:-default}"
ls -la ~/.gup/

# Project information
gup project list
gup status

# Network information
curl -I https://httpbin.org/status/200
```

### Log Files

Enable debug logging for more detailed information:

```bash
# Run with debug logging
RUST_LOG=debug gup add project version

# Check system logs
journalctl -u gup  # systemd systems
grep gup /var/log/syslog  # traditional syslog
```

### Environment Variables

Useful environment variables for debugging:

- `RUST_LOG=debug`: Enable debug logging
- `GUP_DEPOT_PATH`: Override default data directory  
- `GUP_DOWNLOAD_TIMEOUT`: Set download timeout (seconds)
- `HTTP_PROXY`/`HTTPS_PROXY`: Configure proxy settings
- `NO_COLOR=1`: Disable colored output

## Getting Additional Help

If these solutions don't resolve your issue:

1. **Search existing issues**: [GitHub Issues](https://github.com/your-org/gup/issues)
2. **Create new issue**: Include diagnostic information and steps to reproduce
3. **Join discussions**: [GitHub Discussions](https://github.com/your-org/gup/discussions)
4. **Emergency contact**: For critical issues affecting production systems

### Issue Reporting Template

```markdown
**Problem Description**
Brief description of the issue

**Environment**
- OS: 
- GUP version: 
- Shell: 

**Steps to Reproduce**
1. 
2. 
3. 

**Expected Behavior**
What should happen

**Actual Behavior**
What actually happens

**Diagnostic Information**
```
# paste output from diagnostic commands
```

**Additional Context**
Any other relevant information
```