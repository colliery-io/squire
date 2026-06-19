"""Shared helpers for the Squire angreal tasks.

Every task runs from the repo root (``angreal.get_root()`` is the ``.angreal`` dir, so its
``.parent`` is the project root). Subprocesses inherit the caller's environment, so the usual
toolchain (cargo, node/npm) is on PATH; the Gradle helper additionally sources
``clients/android-env.sh`` so ANDROID_HOME / JAVA_HOME are set in non-interactive shells.
"""

import os
import subprocess
from pathlib import Path

import angreal


def root():
    """The project root (parent of the .angreal directory)."""
    return Path(angreal.get_root()).parent


def run(cmd, cwd=None, env=None):
    """Run a list-form command (no shell) at the repo root or ``cwd``. Returns the exit code."""
    merged = {**os.environ, **(env or {})}
    return subprocess.run(cmd, cwd=str(cwd or root()), env=merged).returncode


def bash(script, cwd=None, env=None):
    """Run a bash snippet at the repo root or ``cwd`` (for env-laden / piped commands)."""
    merged = {**os.environ, **(env or {})}
    return subprocess.run(["bash", "-c", script], cwd=str(cwd or root()), env=merged).returncode


def gradle(args):
    """Run ``./gradlew <args>`` in clients/squire-android with the Android SDK/JDK env sourced."""
    android = root() / "clients" / "squire-android"
    return bash(f"source '{root()}/clients/android-env.sh' && ./gradlew {args}", cwd=android)
