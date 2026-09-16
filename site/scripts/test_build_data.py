import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("build_data", Path(__file__).with_name("build-data.py"))
build_data = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build_data)


class ReleaseDataTests(unittest.TestCase):
    def test_published_assets_keep_architecture_and_hash_without_downloading(self):
        release = {"tag_name": "v1.0.5", "assets": [
            {"name": "Docsy_1.0.5_x86-setup.exe", "size": 20, "digest": "sha256:" + "a" * 64},
            {"name": "Docsy_1.0.5_x64-setup.exe", "size": 30},
            {"name": "Docsy_1.0.5_aarch64.dmg", "size": 40},
            {"name": "Docsy_1.0.5_x64-setup.exe.sig", "size": 5},
        ]}
        with tempfile.TemporaryDirectory() as directory, patch.object(build_data, "download_file") as download:
            releases = build_data.build_releases([dict(release, draft=True), dict(release, prerelease=True), release], Path(directory), "", True)
        self.assertEqual(len(releases), 1)
        self.assertEqual([asset["platform"] for asset in releases[0]["assets"]], ["windows-x86", "windows", "macos"])
        self.assertEqual(releases[0]["assets"][0]["sha256"], "a" * 64)
        self.assertIn("/v1.0.5/", releases[0]["assets"][0]["url"])
        download.assert_not_called()


if __name__ == "__main__":
    unittest.main()
