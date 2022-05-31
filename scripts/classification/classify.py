"""Script that preprocesses crates."""

import os
import sys
import subprocess
from multiprocessing.pool import ThreadPool


# We need to exclude crates that:
# 1. Not compile by running "cargo build"
# 2. No FFI functions called
# 3. No entry points detected
# 4. No LLVM bitcode generated
def has_FFI(crate_path):
    """Determine whether a crate has FFI functions."""
    p = subprocess.Popen([
        "../../../target/release/cargo-ffi-checker", "ffi-checker-preprocess"
    ],
                         cwd=crate_path)

    p.communicate()[0]
    rc = p.returncode

    entry_path = os.path.join(crate_path, "target/entry_points")
    bitcode_path = os.path.join(crate_path, "target/bitcode_paths")

    if rc == 0 and os.path.exists(entry_path) and os.path.exists(bitcode_path):
        name = os.path.basename(crate_path)
        if name not in known_crate_list:
            print(name)

    # Clean up
    subprocess.Popen(["cargo", "clean"], cwd=crate_path).wait()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(
            "Need two arguments to specify the path and the size of the thread pool"
        )
        print(
            "Usage example: python -u ./classify.py ../crates 8 > output.txt")
        exit(1)

    # path to the crate directory
    crate_dir = os.path.abspath(sys.argv[1])
    # list of all crates
    crate_list = [os.path.join(crate_dir, i) for i in os.listdir(crate_dir)]

    num_thread = int(sys.argv[2])

    crate_list_file = open("../evaluation/crates_all.txt", 'r').readlines()
    known_crate_list = [
        os.path.basename(line.strip()) for line in crate_list_file
    ]

    with ThreadPool(num_thread) as p:
        p.map(has_FFI, crate_list)
