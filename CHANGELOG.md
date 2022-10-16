# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Note that
the version numbers are *not* semantic.

## UNRELEASED
### Added
* Suspend jobs and resume them afterwards when suspending from the UI.

### Changed
* Don't enqueue a job that's currently running or in the queue.
* Statically link dbus on Linux.

### Updated
* Update bundled restic to 0.14.

## 2.0.0 - 2022-10-14
### Added
* Graceful shutdown handling.
* Clean shutdown on signals.
* Initial suspend UI.
* Option `--log-level` for `daemon` subcommand.

Also I forgot I had a changelog...

## 1.4.1
### Added
* Option `--log-file` for `daemon` subcommand.

### Changed
* `daemon` doesn't create log file by default.
* `daemon --supervisor` does create a log file.

## 1.4.0
### Added
* New DSL for specifying backup schedules.

### Removed
* Cron expression syntax for backup schedules.

## 1.3.3
### Added
* New backup setting `exclude-larger-than`

### Changed
* Support both `-` and `_` as word separator in setting names.

## 1.3.2
### Changed
* Packages are now tar.xz instead of zip

## 1.3.1
### Changed
* Windows: switch back to building with `console` subsystem and using a VBS for autostart

## 1.3.0
### Added
* Extended the `--restic` parameter with special values `system`, `bundled` and `system-then-bundled`.

### Changed
* New tag format: `cirrus.<backup name>`
* Changed `--restic-binary` argument to `--restic`.

## 1.2.1 - 2021-09-08
### Changed
* Windows: build cirrus.exe as combined GUI/CLI application so it doesn't pop up a console window
* Windows: use shortcut instead of VBScript for the startup script
* Move built-in supervisor to a flag on the `daemon` command.

## 1.2.0 - 2021-09-04
### Added
* New built-in self installer that sets up daemon autostart for desktop systems.

### Removed
* Remove cirrus-daemon-watchdog.
* Remove `generate` subcommand.

## 1.1.1 - 2021-09-04
### Changed
* Fix backups run from the daemon if the desktop UI has errored.

## 1.1.0 - 2021-09-01
### Added
* Bundle a built-in daemon supervisor (mostly for Windows).
* Desktop: add menu item to open configuration file

### Removed
* Remove open-config-file CLI command.

## 1.0.1 - 2021-08-08
### Added
* Include target triple in version output.
* Added back Exit menu item to status icon UI.

## 1.0.0 - 2021-08-07
### Added
* Include and show version string in release versions.

### Changed
* Switch to new version number scheme consisting of a manually specified release version
  and a timestamp-based build identifier.
* Update bundled restic to 0.12.1.
* Don't crash if the (Linux) status icon can't load due to missing DBus.

### Removed

## 2021.06.13 - 2021-06-13
A pre-changelog version.
