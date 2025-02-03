# Linetime ⏱

Linetime is a command line utility to add timestamps at the start of lines. The tool can either
process lines from stdin or execute a command and process lines from the command's stdout and
stderr. Unlike the `time` utility, this can be used to wrap a complete script to find bottle necks.

## Basic Usage
Piping to stdin:
```
$ ls -l | linetime
00:00.000: -rw-r--r-- 1 jordf 197609 1104 feb  2 20:11 README.md
00:00.002: ⏱ End
```

Executing a command:
```
$ linetime -- ls -l
00:00.019 stdout: -rw-r--r-- 1 jordf 197609 1104 feb  2 20:11 README.md
00:00.020 ------: ⏱ End
```

Executing a command, with delta times and microsecond precision:
```
$ linetime -d -u -- ls -l
00:00.021819                stdout: -rw-r--r-- 1 jordf 197609 1104 feb  2 20:11 README.md
00:00.023151 (00:00.001331) ------: ⏱ End
```

When a command is executed, linetime will exit with the same code as the executed command. If the
code is not 0, the text `"The command exited with code 1"` or similar will be printed last.

The output from the command is printed to stdout and stderr respectively, as read from the command.
In addition to the timestamp, each line is prefixed with the name of the output file. To avoid
interleaving output from stdout and stderr from an executed command, the output is buffered and
printed first when a complete line is read. This behavior can be disabled with
 `--no-line-buffering` or `-l`, in which case characters are printed as soon as they are read.

See help text, `-h` or `--help`, for a complete list of options.

## Escape Sequences
Some tools use ANSI escape sequences (moving the cursor or erasing lines) to show progress without
moving to a new line in the terminal, or simply carriage return. Many tools disable this behavior
automatically when piping or executed from by another process which is not a terminal. Since this
is not always the case, linetime tries to "unfold" lines that otherwise would have been overwritten.

## Installation
Currently, you have to install your own Rust toolchain with
[`rustup`](https://www.rust-lang.org/tools/install). Then you can install the program with `cargo`
in the repository root:
```
$ cargo install --path .
```

## FAQ

### Why are the timestamps not strictly ordered?
To avoid interleaving output from an executed command's stdout and stderr, lines are buffered and
only printed when a lines is complete. However, the timestamp is taken when the first character is
read on the line.
