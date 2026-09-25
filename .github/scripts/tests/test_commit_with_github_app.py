"""Exercise App commit creation against real local Git objects and a mocked API."""

import base64
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "commit-with-github-app.py"
spec = importlib.util.spec_from_file_location("app_commit", SCRIPT)
app_commit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(app_commit)


class AppCommitTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        old_cwd = Path.cwd()
        os.chdir(self.temp.name)
        self.addCleanup(os.chdir, old_cwd)
        self.env = patch.dict(os.environ, {
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GITHUB_REPOSITORY": "example/repo",
        })
        self.env.start()
        self.addCleanup(self.env.stop)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "App")
        self.git("config", "user.email", "app@example.com")
        Path("fragment").write_text("consumed fragment")
        Path("rename me").write_text("renamed file")
        Path("version").write_text("old")
        self.git("add", ".")
        self.git("commit", "-m", "base")
        self.parent = self.git("rev-parse", "HEAD").strip().decode()
        Path("fragment").unlink()
        Path("rename me").rename("new name")
        Path("version").write_text("new")
        Path("binary").write_bytes(b"\x00\xff\x01")
        Path("executable").write_text("#!/bin/sh\nexit 0\n")
        Path("executable").chmod(0o755)
        Path("link").symlink_to("version")
        self.git("add", "-A")
        self.tree = self.git("write-tree").strip().decode()
        self.commit_payload = None
        self.verified = True
        self.wrong_tree = False

    def git(self, *args, data=None, env=None):
        return subprocess.check_output(["git", *args], input=data, env=env, stderr=subprocess.DEVNULL)

    def api(self, endpoint, payload):
        if endpoint.endswith("/blobs"):
            sha = self.git("hash-object", "-w", "--stdin", data=base64.b64decode(payload["content"]))
        elif endpoint.endswith("/trees"):
            env = dict(os.environ, GIT_INDEX_FILE=str(Path.cwd() / "api-index"))
            self.git("read-tree", payload["base_tree"], env=env)
            for entry in payload["tree"]:
                if entry["sha"] is None:
                    self.git("update-index", "--force-remove", "--", entry["path"], env=env)
                else:
                    self.git("update-index", "--add", "--cacheinfo", entry["mode"], entry["sha"], entry["path"], env=env)
            sha = self.git("write-tree", env=env)
            if self.wrong_tree:
                sha = b"0" * 40
        else:
            self.commit_payload = payload
            sha = self.git("commit-tree", payload["tree"], "-p", payload["parents"][0], "-m", payload["message"])
            return {"sha": sha.strip().decode(), "verification": {"verified": self.verified}}
        return {"sha": sha.strip().decode()}

    def execute(self):
        def local_git(*args):
            # The mocked API creates objects locally, so no network fetch is needed.
            return b"" if args[0] == "fetch" else self.git(*args)

        with patch.object(app_commit, "api", self.api), patch.object(app_commit, "git", local_git), patch("sys.argv", [str(SCRIPT), "Release artifacts"]):
            app_commit.main()

    def test_exact_staged_tree_and_app_attribution(self):
        self.execute()
        self.assertEqual(self.git("rev-parse", "HEAD^{tree}").strip().decode(), self.tree)
        self.assertEqual(self.git("rev-parse", "HEAD^").strip().decode(), self.parent)
        self.assertEqual(set(self.commit_payload), {"message", "tree", "parents"})
        self.assertEqual(self.git("diff", "--cached"), b"")

    def test_unverified_commit_does_not_advance_head(self):
        self.verified = False
        with self.assertRaisesRegex(SystemExit, "did not verify"):
            self.execute()
        self.assertEqual(self.git("rev-parse", "HEAD").strip().decode(), self.parent)

    def test_tree_mismatch_does_not_create_commit(self):
        self.wrong_tree = True
        with self.assertRaisesRegex(SystemExit, "tree differs"):
            self.execute()
        self.assertIsNone(self.commit_payload)
        self.assertEqual(self.git("rev-parse", "HEAD").strip().decode(), self.parent)

    def test_empty_index_does_not_create_commit(self):
        self.git("read-tree", "HEAD")
        with self.assertRaisesRegex(SystemExit, "No staged changes"):
            self.execute()
        self.assertIsNone(self.commit_payload)


if __name__ == "__main__":
    unittest.main()
