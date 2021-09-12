This library provides tools for building an installer included directly in your binary.

When distributing programs as a single binary, system integration can become tricky. This includes things like
installing desktop launchers, autostart files, and other global configuration. A common solution is an installer script,
but this library is designed for an alternative: still distribute the program as a single binary, but include an
installer in the binary so it can be run after installing the binary itself.

Features:
* several built-in installation commands
* show installation progress
* uninstaller
* show detailed installation steps to take
* installation into a destination directory instead of the system root
