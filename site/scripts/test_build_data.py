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

    def test_static_download_tracks_latest_published_release(self):
        html = '''<strong id="dl-version-static">v1.0.0</strong>
<code id="dl-mac-name">Docsy_1.0.0_aarch64.dmg</code>
<code id="dl-win-name">Docsy_1.0.0_x64-setup.exe</code>
<a id="dl-win-x86" href="https://example.com/old"></a>
<a id="dl-release-page" href="https://example.com/old"></a>'''
        releases = [{
            "version": "1.1.4",
            "tag": "v1.1.4",
            "assets": [
                {"platform": "macos", "name": "Docsy_1.1.4_aarch64.dmg"},
                {"platform": "windows", "name": "Docsy_1.1.4_x64-setup.exe"},
                {"platform": "windows-x86", "name": "Docsy_1.1.4_x86-setup.exe"},
            ],
        }]
        with tempfile.TemporaryDirectory() as directory:
            index = Path(directory) / "index.html"
            index.write_text(html, encoding="utf-8")
            build_data.sync_static_download(index, releases)
            synced = index.read_text(encoding="utf-8")
        self.assertIn('id="dl-version-static">v1.1.4<', synced)
        self.assertIn('id="dl-mac-name">Docsy_1.1.4_aarch64.dmg<', synced)
        self.assertIn('id="dl-win-name">Docsy_1.1.4_x64-setup.exe<', synced)
        self.assertNotIn("example.com/old", synced)
        self.assertEqual(synced.count("/tag/v1.1.4"), 2)


if __name__ == "__main__":
    unittest.main()
