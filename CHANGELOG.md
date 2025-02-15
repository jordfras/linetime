# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [1.0.2] - 2025-02-15

### Changed
- For long running commands/input, timestamps and delta times are expanded with an hour field when
  necessary.


## [1.0.1] - 2025-02-04

### Added
- Script to demonstrate unfolding.

### Fixed
- Improved README.md to be more clear and correct.


## [1.0.0] - 2025-02-03

### Added
- Ability to add timestamp prefix on lines read on stdin or from stdout/stderr of an executed
  command.
- Ability to also add delta time between lines.
- Possibility to increase resolution from milliseconds to microseconds.
- When executing command, ensuring exiting with the same exit code as the command.
- Printing a line with the exit code unless it is 0.
- "Unfolding" of lines hidden with repeated use of carriage return or ANSI escape code to move
  cursor or erase characters from the terminal.
- Buffering of lines to ensure stderr and stdout of executed command are not interleaved.
- Lots and lots of unit test and integration tests with a clever marionette program control the
  executed command.
