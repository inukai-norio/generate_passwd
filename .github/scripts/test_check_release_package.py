#!/usr/bin/env python3
# SPDX-License-Identifier: ISC
# Copyright 2026 inukai-norio <inukay@proton.me>
# License text: TEST_CODE_LICENSE
"""配布ターゲットごとの必須ファイルと、ZIP内のmusl表示を検証する。"""

import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


CHECKER = Path(__file__).with_name("check_release_package.py").resolve()
LINUX_TARGETS = (
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "i686-unknown-linux-musl",
)
OTHER_TARGETS = (
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "i686-pc-windows-msvc",
)


class ReleasePackageTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ("README.md", "TEST_CODE_LICENSE", "license-docs/LICENSES.html"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"Fixture for {name}\n", encoding="utf-8")
        (self.root / "MUSL_LICENSE").write_text(
            "Copyright © musl fixture\n", encoding="utf-8"
        )

    def binary(self, target):
        name = "generate_passwd.exe" if "windows" in target else "generate_passwd"
        path = self.root / "target" / target / "release" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"binary fixture\n")
        return path

    def run_check(self, target, archive=None):
        command = [
            sys.executable,
            str(CHECKER),
            "--target", target,
            "--binary", str(self.binary(target)),
        ]
        if archive is not None:
            command.extend(["--archive", str(archive)])
        return subprocess.run(command, cwd=self.root, capture_output=True, text=True)

    def archive(self, target, *, musl=None, extra=None):
        path = self.root / "package.zip"
        files = [
            self.binary(target),
            self.root / "README.md",
            self.root / "TEST_CODE_LICENSE",
            self.root / "license-docs/LICENSES.html",
        ]
        with zipfile.ZipFile(path, "w") as archive:
            for file in files:
                archive.write(file, file.name)
            if musl is not None:
                archive.writestr("MUSL_LICENSE", musl)
            if extra is not None:
                archive.writestr(extra, b"extra fixture")
        return path

    def test_all_release_targets(self):
        cases = [(t, 5) for t in LINUX_TARGETS] + [(t, 4) for t in OTHER_TARGETS]
        for target, count in cases:
            with self.subTest(target=target):
                musl = (
                    (self.root / "MUSL_LICENSE").read_bytes() if count == 5 else None
                )
                archive = self.archive(target, musl=musl)
                for result in (self.run_check(target), self.run_check(target, archive)):
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn(f"Verified {count} package files:", result.stdout)

    def test_linux_rejects_missing_or_empty_musl_input(self):
        license_file = self.root / "MUSL_LICENSE"
        for target in LINUX_TARGETS:
            for empty in (False, True):
                with self.subTest(target=target, empty=empty):
                    license_file.unlink(missing_ok=True)
                    if empty:
                        license_file.touch()
                    result = self.run_check(target)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("missing or empty: MUSL_LICENSE", result.stderr)

    def test_other_targets_do_not_require_musl_input(self):
        (self.root / "MUSL_LICENSE").unlink()
        for target in OTHER_TARGETS:
            with self.subTest(target=target):
                result = self.run_check(target, self.archive(target))
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_linux_rejects_missing_or_changed_musl_in_zip(self):
        for target in LINUX_TARGETS:
            for musl in (None, b"changed notice"):
                with self.subTest(target=target, musl=musl):
                    result = self.run_check(target, self.archive(target, musl=musl))
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("MUSL_LICENSE", result.stderr)

    def test_other_targets_reject_extra_musl_in_zip(self):
        for target in OTHER_TARGETS:
            with self.subTest(target=target):
                result = self.run_check(target, self.archive(target, musl=b"extra notice"))
                self.assertEqual(result.returncode, 1)
                self.assertIn("Unexpected ZIP entries", result.stderr)

    def test_linux_rejects_extra_or_nested_zip_entries(self):
        target = LINUX_TARGETS[0]
        for entry in ("extra.txt", "docs/MUSL_LICENSE"):
            with self.subTest(entry=entry):
                musl = (
                    (self.root / "MUSL_LICENSE").read_bytes()
                    if entry == "extra.txt" else None
                )
                archive = self.archive(target, musl=musl, extra=entry)
                result = self.run_check(target, archive)
                self.assertEqual(result.returncode, 1)
                self.assertIn("Unexpected ZIP entries", result.stderr)


if __name__ == "__main__":
    unittest.main()
