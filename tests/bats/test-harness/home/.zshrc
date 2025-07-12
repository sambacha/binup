#!/bin/zsh
# Test harness .zshrc - Realistic sample configuration

# Path to oh-my-zsh installation (example)
export ZSH="$HOME/.oh-my-zsh"

# Set theme
ZSH_THEME="robbyrussell"

# Which plugins would you like to load?
plugins=(git docker kubectl)

# User configuration
export LANG=en_US.UTF-8
export EDITOR='vim'

# Compilation flags
export ARCHFLAGS="-arch x86_64"

# Custom aliases
alias zshconfig="vim ~/.zshrc"
alias ohmyzsh="vim ~/.oh-my-zsh"
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'

# Example PATH modifications (before gup)
path=(
    $HOME/bin
    $HOME/.local/bin
    /usr/local/bin
    $path
)

# Export PATH
export PATH

# Load oh-my-zsh (if it exists)
[[ -f $ZSH/oh-my-zsh.sh ]] && source $ZSH/oh-my-zsh.sh

# Node Version Manager
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Rust/Cargo
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

# Custom functions
mkcd() {
    mkdir -p "$1" && cd "$1"
}

# Homebrew (on macOS)
if [[ -f "/opt/homebrew/bin/brew" ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
fi

# End of .zshrc