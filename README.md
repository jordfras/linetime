# Linetime ⏱

Linetime is a command utility to add timestamps at the start of lines. The tool can either process
lines from stdin or execute a command and process lines from the command's stdout and stderr.

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

See help text, `-h` or `--help`, for a complete list of options.

## Features
To avoid interleaving output from stdout and stderr from an executed command, the output is
buffered and printed first when a complete line is read. This behavior can be disabled with
 `--no-line-buffering` or `-l`, in which case characters are printed as soon as they are read.

Some tools use ANSI escape sequences (moving the cursor or erasing lines) to show progress without
moving to a new line in the terminal. Many tools disable this behaviour automatically when piping
or executed from by another process which is not a terminal. Since this is not always the case,
linetime tries to "unfold" lines that otherwise would have been overwritten.

## FAQ

### Why is the timestamp not strictly ordered?
To avoid interleaving output from an executed command's stdout and stderr, lines are buffered and
only printed when a lines is complete. However, the timestamp is taken when the first character is
read on the line.
