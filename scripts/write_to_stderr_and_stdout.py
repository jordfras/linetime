#!/usr/bin/env python3

import sys
import time

if __name__ == "__main__":
    for l in range(3):
        for c in range(10):
            sys.stderr.write("e")
            sys.stderr.flush()
            sys.stdout.write("o")
            sys.stdout.flush()
            time.sleep(0.20)
        sys.stderr.write("\n")
        sys.stderr.flush()
        sys.stdout.write("\n")
        sys.stdout.flush()
