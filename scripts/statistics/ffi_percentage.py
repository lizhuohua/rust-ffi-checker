"""Script that crawls the reverse-dependencies of all crates from crates.io."""

import requests
import pickle


def get_rev_deps(crate_name, cache):
    """Return a list of crates that depend on `crate_name`."""
    # First try to look it up in cache
    value = cache.get(crate_name)
    if value is not None:
        return value

    # If not found in cache, request crates.io
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
            deps_list.add(dep['crate'])
    return deps_list


def make_ffi_crate_list():
    """Return a list of crate names that are of category 'api-bindings'."""
    request_page = requests.get(
        'https://crates.io/api/v1/crates?category=external-ffi-bindings')
    crate_total_num = int(request_page.json()['meta']['total'])
    # 100 crates per page, take the ceiling
    page_list = list(range(1, (crate_total_num + 100 - 1) // 100 + 1))
    crate_name_list = set()
    for page in page_list:
        request_page = requests.get(
            'https://crates.io/api/v1/crates?category=api-bindings&page={}&per_page=100&sort=downloads'
            .format(page))
        for crate in request_page.json()['crates']:
            crate_name_list.add(crate['name'])

    return crate_name_list


def make_reverse_deps_map(crate_name_list, cache):
    """Return a map that maps each crate in `crate_name_list` to its reverse dependencies."""
    reverse_deps_map = {}
    for index, crate_name in enumerate(crate_name_list):
        if index % 100 == 0:
            print("processed:", index)
        reverse_deps_map[crate_name] = get_rev_deps(crate_name, cache)
    return reverse_deps_map


def load_reverse_deps_map():
    """Load the reverse dependency map."""
    # f1 = open("1_layer_deps_map.pickle", "rb")
    # deps_map1 = pickle.load(f1)
    # f1.close()
    # f2 = open("2_layer_deps_map.pickle", "rb")
    # deps_map2 = pickle.load(f2)
    # f2.close()
    # f3 = open("3_layer_deps_map.pickle", "rb")
    # deps_map3 = pickle.load(f3)
    # f3.close()

    # deps_map = deps_map1 | deps_map2 | deps_map3
    f3 = open("9_layer_deps_map.pickle", "rb")
    deps_map = pickle.load(f3)
    f3.close()

    return deps_map


if __name__ == '__main__':
    print("Loading caches...")
    deps_map_cache = load_reverse_deps_map()

    print("Requesting the API of crates.io...")
    ffi_crate_list = make_ffi_crate_list()
    print("API bindings crate:", len(ffi_crate_list))
    all_crates_set = set(ffi_crate_list)
    current_crates_set = set(ffi_crate_list)
    layer = 1
    while True:
        old_size = len(all_crates_set)
        deps_map = make_reverse_deps_map(current_crates_set, deps_map_cache)
        deps_map_cache = deps_map_cache | deps_map

        # Dump useful variables for future use
        f = open(str(layer) + "_layer_deps_map_external_ffi.pickle", "wb")
        pickle.dump(deps_map, f)
        f.close()

        current_crates_set = set()
        for deps_set in deps_map.values():
            current_crates_set = current_crates_set.union(deps_set)
        print(layer, "layer dependencies:", len(current_crates_set))
        all_crates_set = all_crates_set.union(current_crates_set)
        print("Total crates:", len(all_crates_set))
        if len(all_crates_set) == old_size:
            break
        layer = layer + 1
