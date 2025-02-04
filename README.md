# Linetime ⏱

Linetime is a command line utility to add timestamps at the start of lines. The tool can either
process lines from stdin or execute a command and process lines from the command's stdout and
stderr. It can, for instance, be useful to find performance bottle necks in a script.

## Usage
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
interleaving output from stdout and stderr, the output is buffered and printed first when a complete
line is read. This behavior can be disabled with `--no-line-buffering` or `-l`, in which case
characters are printed as soon as they are read.

See help text, `-h` or `--help`, for a complete list of options.

## Unfolding
Some tools show progress without starting a new line in the terminal. The cursor is moved or lines
are erased by printing carriage return and/or ANSI escape sequences. Many tools disable this
behavior automatically when piping or executed by another process. Not all tools are this
well-behaved so linetime tries to "unfold" lines that otherwise would have been overwritten.

Unfolding of output from `cargo` can be demonstrated if you clone the linetime git repository,
perform a release build and run a demonstration script: 
```
# Make a release build first to allow script to use a release binary.
$ cargo build --release

# Demonstration script builds linetime twice, with and without piping output to linetime.
$ scripts/demo_unfolding.sh
```

## Installation
Currently, you have to install your own Rust toolchain with
[`rustup`](https://www.rust-lang.org/tools/install). Then you can download, build and install the
program with a single `cargo` command:
```
$ cargo install linetime
```

## FAQ

### Why are the timestamps not strictly ordered?
To avoid interleaving output from an executed command's stdout and stderr, lines are buffered and
only printed when a lines is complete. However, the timestamp is taken when the first character is
read on the line.
