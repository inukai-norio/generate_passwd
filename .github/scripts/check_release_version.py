#!/usr/bin/env python3
# SPDX-License-Identifier: ISC
# Copyright 2026 inukai-norio <inukay@proton.me>
# License text: TEST_CODE_LICENSE
"""2つのコミットのpackage.versionを比較し、GitHub Actionsの出力へ書く。"""

import os
import subprocess
import tomllib


def package_version(revision):
    manifest = subprocess.check_output(
        ["git", "show", f"{revision}:Cargo.toml"], text=True
    )
    version = tomllib.loads(manifest)["package"]["version"]
    if not isinstance(version, str) or not version:
        raise ValueError("package.version must be a non-empty string")
    return version


def main():
    before = package_version(os.environ["BEFORE_SHA"])
    after = package_version(os.environ["AFTER_SHA"])
    changed = str(before != after).lower()
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"changed={changed}\n")
        output.write(f"version={after}\n")
    print(f"package.version: {before} -> {after} (changed={changed})")


if __name__ == "__main__":
    main()
