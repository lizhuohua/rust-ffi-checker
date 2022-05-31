"""Script that runs evaluation in parallel."""

import os
import sys
import signal
import subprocess
import threading
from multiprocessing.pool import ThreadPool
import csv
import re


class bcolors:
    """Define colors for pretty outputs."""

    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


# path to the current script
root_dir = os.path.dirname(os.path.abspath(__file__))
# path to the output directory
output_dir = os.path.join(root_dir, "outputs")
# path to the cargo sub-command
executable = os.path.join(root_dir, "../../target/release/cargo-ffi-checker")

lock = threading.Lock()
count = 0


def evaluate(crate_dir):
    """Evaluate a crate given its path."""
    crate_name = os.path.basename(crate_dir)
    print("Evaluating", crate_name)

    # Run `cargo clean` to make sure it does not use cache
    subprocess.Popen(["cargo", "clean"], cwd=crate_dir).wait()

    # Use `time` command to get execution time and peak memory usage
    with subprocess.Popen(
        ["/usr/bin/time", "-f", "%M\n%e", executable, "ffi-checker"],
            cwd=crate_dir,
            stdout=subprocess.PIPE,
            # stderr=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            preexec_fn=os.setsid) as process:
        try:
            # Analysis results are in stdout, execution time etc. are in stderr
            out, err = process.communicate(timeout=timeout_sec)

            out_str = out.decode("utf-8")
            err_str = err.decode("utf-8").split()
            if out_str != "":
                print(bcolors.OKGREEN, "Bug Detected!", bcolors.ENDC)
                print(bcolors.OKGREEN, crate_name, bcolors.ENDC)
                print(bcolors.OKGREEN, out_str, bcolors.ENDC)

            elasp_time = float(err_str[-1])
            peak_mem = int(err_str[-2])

            if lock.acquire():
                global count
                count += 1
                print("Progress:", count, "/", total_count)
                lock.release()

            if process.returncode == 0:
                print(bcolors.OKBLUE, "Finish analyzing crate", crate_name,
                      bcolors.ENDC)

                print("Cleaning up")
                subprocess.Popen(["cargo", "clean"], cwd=crate_dir).wait()
                return [out_str, elasp_time, peak_mem]

            else:
                print(bcolors.FAIL, "Error while analyzing crate", crate_name,
                      bcolors.ENDC)
                print("Cleaning up")
                subprocess.Popen(["cargo", "clean"], cwd=crate_dir).wait()
                return ["", elasp_time, peak_mem]

        except subprocess.TimeoutExpired:
            print(bcolors.FAIL, "Timeout while analyzing crate", crate_name,
                  bcolors.ENDC)

            if lock.acquire():
                count += 1
                print("Progress:", count, "/", total_count)
                lock.release()
            # send signal to the process group
            os.killpg(process.pid, signal.SIGTERM)

            print("Cleaning up")
            subprocess.Popen(["cargo", "clean"], cwd=crate_dir).wait()
            return ["", 0, 0]


def mkdir(dir_name):
    """Create a directory (if it does not exist) in the current directory."""
    if not os.path.exists(dir_name):
        os.makedirs(dir_name)


if __name__ == "__main__":
    if len(sys.argv) != 5:
        print(
            "Need four arguments to specify the crate list, the crate directory, the size of the thread pool, and the timeout in seconds"
        )
        print(
            "Usage example: `python evaluate.py crate_list.txt ../crates_with_bugs 8 240`"
        )
        exit(1)

    # Read crate list that will be analyzed
    crate_list_file = open(sys.argv[1], 'r').readlines()
    crate_list = [line.strip() for line in crate_list_file]
    total_count = len(crate_list)
    print(crate_list)

    # path to the crate directory
    crate_dir = os.path.join(root_dir, sys.argv[2])
    # paths to the all test cases
    test_cases_dir = [os.path.join(crate_dir, i) for i in crate_list]

    # Read the size of the thread pool
    num_thread = int(sys.argv[3])
    print(len(crate_list), "tasks in total, run in", num_thread, "threads")

    # Read the timeout limit
    timeout_sec = int(sys.argv[4])

    mkdir(output_dir)
    os.chdir(output_dir)

    results = []

    p = ThreadPool(num_thread)
    for i in range(0, len(crate_list)):
        results.append(p.apply_async(evaluate, args=(test_cases_dir[i], )))
    p.close()
    p.join()
    results = {crate_list[i]: r.get() for i, r in enumerate(results)}

    # Dump results in CSV format
    with open('result.csv', 'w', newline='') as csvfile:
        csvwriter = csv.writer(csvfile,
                               delimiter=',',
                               quotechar='|',
                               quoting=csv.QUOTE_MINIMAL)
        csvwriter.writerow(["Package", "high", "mid", "low", "time", "memory"])
        for k, v in results.items():
            diagnoses = v[0].split("\n")[:-1]
            low = 0
            mid = 0
            high = 0
            for diagnosis in diagnoses:
                m = re.search(", seriousness: ...", diagnosis)
                severity = m.group(0)[-3:]
                if severity == "Low":
                    low += 1
                elif severity == "Med":
                    mid += 1
                else:
                    high += 1
            csvwriter.writerow([
                k,
                str(high),
                str(mid),
                str(low),
                str(v[1]),
                str(v[2] / 1024)
            ])

    # Print and dump result
    result_file = open('result.txt', 'w')
    print("Results:")
    for k, v in results.items():
        if v != "":
            print(k + ":")
            print(v)
            result_file.write(k + ":\n")
            diagnoses = v[0].split("\n")[:-1]
            for diagnosis in diagnoses:
                result_file.write(diagnosis + "\n")
            result_file.write(str(v[1]) + "\n")
            result_file.write(str(v[2]) + "\n\n")

    result_file.close()
