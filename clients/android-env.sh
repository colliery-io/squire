#!/usr/bin/env bash
# Source this to get the Android SDK + JDK on your environment in a non-interactive shell
# (CI, scripts). Interactive zsh already gets this from the managed block in ~/.zshrc.
#
#   source clients/android-env.sh
#
# Installed out-of-band (not in MacPorts): the Android command-line SDK lives in
# ~/Library/Android/sdk and Gradle 8.x is symlinked into ~/.local/bin. See clients/README.md.

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
# AGP wants JDK 17; prefer it, fall back to the default JDK.
export JAVA_HOME="$(/usr/libexec/java_home -v 17 2>/dev/null || /usr/libexec/java_home)"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$HOME/.local/bin:$PATH"
