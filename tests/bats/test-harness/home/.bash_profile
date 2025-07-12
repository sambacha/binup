#!/bin/bash
# Test harness .bash_profile - Loaded for login shells

# Source .profile for common settings
if [ -f "$HOME/.profile" ]; then
    . "$HOME/.profile"
fi

# Source .bashrc for interactive settings
if [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi

# Login-specific settings
echo "Last login: $(date)" > ~/.last_login

# End of .bash_profile