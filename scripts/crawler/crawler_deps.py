"""Script that crawls the reverse-dependencies of a crate from crates.io."""

import requests
import sys
from subprocess import Popen, DEVNULL


def get_rev_deps(crate_name):
    """Return a list of download links that depend on `crate_name`."""
    request_page = requests.get(
        'https://crates.io/api/v1/crates/{}/reverse_dependencies'.format(
            crate_name))
    deps_total_num = int(request_page.json()['meta']['total'])
    # 100 crates per page, take the ceiling
    page_list = list(range(1, (deps_total_num + 100 - 1) // 100 + 1))
    deps_list = set()
    for page in page_list:
        request_page = requests.get(
            'https://crates.io/api/v1/crates/{}/reverse_dependencies?page={}&per_page=100'
            .format(crate_name, page))
        for dep in request_page.json()['versions']:
            deps_list.add('https://crates.io' + dep['dl_path'])
    return deps_list


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Need two arguments to specify a package name and a path")
        print("Usage example: `python cc ../crates`")
        exit(1)

    crate_name = sys.argv[1]
    download_path = sys.argv[2]

    print("Requesting the API of crates.io...")
    dl_path_list = get_rev_deps(crate_name)
    for dl_path in dl_path_list:
        Popen(
            ["wget", dl_path],
            cwd=download_path,
        ).wait()
        Popen(
            ["tar", "xzf", "download"],
            cwd=download_path,
        ).wait()
        Popen(
            ["rm", "download"],
            cwd=download_path,
        ).wait()
        print("Finish", dl_path)
