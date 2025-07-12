#!/bin/sh
# Test harness .profile - POSIX compatible configuration

# Set up the environment
export LANG=en_US.UTF-8
export EDITOR=vi
export PAGER=less

# Set PATH so it includes user's private bin if it exists
if [ -d "$HOME/bin" ] ; then
    PATH="$HOME/bin:$PATH"
fi

# Set PATH so it includes user's private bin if it exists
if [ -d "$HOME/.local/bin" ] ; then
    PATH="$HOME/.local/bin:$PATH"
fi

# Basic aliases (POSIX compatible)
alias ll='ls -l'
alias la='ls -la'

# If running bash, include .bashrc if it exists
if [ -n "$BASH_VERSION" ]; then
    if [ -f "$HOME/.bashrc" ]; then
        . "$HOME/.bashrc"
    fi
fi

# Example of existing software setup
if [ -d "/opt/local/bin" ]; then
    PATH="/opt/local/bin:$PATH"
fi

# End of .profile