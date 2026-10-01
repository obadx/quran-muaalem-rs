"""Smoke test for the BoltFFI Python binding. Run via tests/bindings/run.sh python."""

import os
import unittest

import quran_muaalem_engine as engine


class VersionTest(unittest.TestCase):
    def test_version_matches_crate(self):
        version = engine.version()
        self.assertIsInstance(version, str)
        self.assertRegex(version, r"^\d+\.\d+\.\d+")

        expected = os.environ.get("ENGINE_VERSION")
        if expected:
            self.assertEqual(version, expected)


if __name__ == "__main__":
    unittest.main()
