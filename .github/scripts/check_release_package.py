#!/usr/bin/env python3
# SPDX-License-Identifier: ISC
# Copyright 2026 inukai-norio <inukay@proton.me>
# License text: TEST_CODE_LICENSE
"""リリースの入力ファイルと、ZIP内の配置・内容を確認する。"""

import argparse
import sys
import zipfile
from pathlib import Path


def package_files(binary, target):
    files = [
        binary,
        Path("README.md"),
        Path("TEST_CODE_LICENSE"),
        Path("license-docs/LICENSES.html"),
    ]
    if target.endswith("-linux-musl"):
        files.append(Path("MUSL_LICENSE"))
    for path in files:
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"Required package file is missing or empty: {path}")
    return files


def check_archive(path, files):
    with zipfile.ZipFile(path) as archive:
        expected = {file.name for file in files}
        names = archive.namelist()
        if len(names) != len(files) or set(names) != expected:
            raise ValueError(
                f"Unexpected ZIP entries: {names}; expected: {sorted(expected)}"
            )
        for file in files:
            if archive.read(file.name) != file.read_bytes():
                raise ValueError(f"ZIP content differs from package input: {file.name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--archive", type=Path)
    args = parser.parse_args()
    try:
        files = package_files(args.binary, args.target)
        if args.archive is not None:
            check_archive(args.archive, files)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(error, file=sys.stderr)
        return 1
    print(f"Verified {len(files)} package files: {args.archive or 'inputs'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
