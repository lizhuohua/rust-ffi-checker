"""Script that crawls the reverse-dependencies of all "external-ffi-bindings" crates from crates.io."""

import requests
from subprocess import Popen


def get_rev_deps(crate_name):
    """Return a list of download links of crates that depend on `crate_name`."""
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


def make_ffi_crate_list():
    """Return a list of crate names that are of category 'external-ffi-bindings'."""
    request_page = requests.get(
        'https://crates.io/api/v1/crates?category=external-ffi-bindings')
    crate_total_num = int(request_page.json()['meta']['total'])
    # 100 crates per page, take the ceiling
    page_list = list(range(1, (crate_total_num + 100 - 1) // 100 + 1))
    crate_name_list = set()
    for page in page_list:
        request_page = requests.get(
            'https://crates.io/api/v1/crates?category=external-ffi-bindings&page={}&per_page=100&sort=downloads'
            .format(page))
        for crate in request_page.json()['crates']:
            crate_name_list.add(crate['name'])

    return crate_name_list


if __name__ == '__main__':
    print("Requesting the API of crates.io...")
    ffi_crate_list = make_ffi_crate_list()
    num_ffi_crate = len(ffi_crate_list)
    print("Numer of FFI crates: ", num_ffi_crate)
    exit(1)
    for i, ffi_crate in enumerate(ffi_crate_list):
        print("Downloading reverse dependencies... (", i, "/", num_ffi_crate,
              ")")
        rev_deps_list = get_rev_deps(ffi_crate)
        print("Number of deps for this crate: ", len(rev_deps_list))
        for dl_path in rev_deps_list:
            Popen(
                ["wget", dl_path],
                cwd="../crates",
            ).wait()
            Popen(
                ["tar", "xzf", "download"],
                cwd="../crates",
            ).wait()
            Popen(
                ["rm", "download"],
                cwd="../crates",
            ).wait()
