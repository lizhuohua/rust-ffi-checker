"""Script that crawls crates from crates.io."""

import requests
import os
import pickle
from subprocess import Popen, DEVNULL

# Global variable that stores the repository addresses that have been processed
repo_set = set()


def clone_repo(name, repo):
    """Clone a repository from address `repo` and rename the directory as `name`.

    Return `True` if succeed, `False` if failed.
    """
    # crates.io API may not always correctly return the repository address
    if repo is None:
        print("Warning:", name,
              "is ignored because its repository address is none")
        return False

    global repo_set
    if repo in repo_set:
        # Different crates on crates.io may have the same repository address
        # Do not clone repositories that have already been cloned
        print("Warning:", name,
              "is ignored because it has already been cloned from", repo)
        return False
    else:
        repo_set.add(repo)
        print("Cloning repo: ", name, "from:", repo)
        my_env = os.environ.copy()
        my_env[
            "GIT_TERMINAL_PROMPT"] = "0"  # Some repositories need username and password, use this to fail instead of prompting for credentials
        p = Popen(["git", "clone", "--depth=1", repo, name],
                  cwd="../crates",
                  stdout=DEVNULL,
                  stderr=DEVNULL,
                  env=my_env)
        p.communicate()[0]
        if p.returncode != 0:
            print("Warning: Error whiling cloning repo:", repo)
            return False
        return True


def make_crate_list():
    """Return a list of crates according to the category and page lists."""
    request_page = requests.get('https://crates.io/api/v1/crates')
    crate_total_num = int(request_page.json()['meta']['total'])
    # 100 crates per page, take the ceiling
    page_list = list(range(1, (crate_total_num + 100 - 1) // 100 + 1))
    crate_list = []
    for page in page_list:
        request_page = requests.get(
            'https://crates.io/api/v1/crates?&page={}&per_page=100&sort=downloads'
            .format(page))
        crate_list += request_page.json()['crates']
    f = open("crate_list.pickle", "wb")
    pickle.dump(crate_list, f)
    f.close()
    return crate_list


def load_crate_list():
    """Load the crate list."""
    f = open("crate_list.pickle", "rb")
    crate_list = pickle.load(f)
    f.close()
    return crate_list


def should_ignore(name, description):
    """Determine whether a crate should be ignored according to its name and description."""
    description = description.lower()
    # Exclude crates that are related to FFI, macro/trait definitions, multi-threads, etc.
    keywords = [
        "ffi", "macro", "binding", "wrapper", "float", "api", "abi", "trait",
        "concurrent", "async", "pin", "mutex", "lock", "atomic", "thread",
        "string", "rational", "libm", "cortex", "hal", "simd", "asm", "sys",
        "stm32", "arch", "gpio"
    ]
    if any([keyword in name + description for keyword in keywords]):
        print("Warning:", name, "is ignored because it is not our concern")
        return True
    return False


if __name__ == '__main__':
    count = 0  # The number of crates successfully cloned
    # print("Requesting the API of crates.io...")
    # crate_list = make_crate_list()  # uncomment this to update the crate list
    print("Loading crate list...")
    crate_list = load_crate_list()
    print("Got addresses of {} crates, start cloning...".format(
        len(crate_list)))
    for crate in crate_list:
        if count >= 200:
            break
        name = crate['name']
        description = crate['description']
        repo = crate['repository']

        if clone_repo(name, repo):
            count += 1

    print(count, "crates cloned")
