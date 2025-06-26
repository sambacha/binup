# Test harness config.fish - Fish shell configuration

# Set environment variables
set -x EDITOR vim
set -x LANG en_US.UTF-8

# Add user bin to PATH
if test -d "$HOME/bin"
    set -x PATH "$HOME/bin" $PATH
end

# Aliases
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'

# Custom functions
function mkcd
    mkdir -p $argv[1]; and cd $argv[1]
end

# Example existing PATH modification
if test -d "$HOME/.local/bin"
    set -x PATH "$HOME/.local/bin" $PATH
end

# End of config.fish