#!/bin/bash
# Test harness .bashrc - Realistic sample configuration

# Source global definitions
if [ -f /etc/bashrc ]; then
    . /etc/bashrc
fi

# User specific environment
export EDITOR=vim
export LANG=en_US.UTF-8

# User specific aliases and functions
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'
alias grep='grep --color=auto'

# Add user's private bin to PATH
if [ -d "$HOME/bin" ] ; then
    PATH="$HOME/bin:$PATH"
fi

# Custom prompt
PS1='[\u@\h \W]\$ '

# History settings
HISTCONTROL=ignoreboth
HISTSIZE=1000
HISTFILESIZE=2000

# Enable programmable completion
if [ -f /etc/bash_completion ] && ! shopt -oq posix; then
    . /etc/bash_completion
fi

# Custom functions
mkcd() {
    mkdir -p "$1" && cd "$1"
}

# Node Version Manager (example of existing PATH modification)
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Rust/Cargo
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

# End of .bashrc