#!/usr/bin/env python3
"""Exercise archive capability validation with controlled signature fixtures."""
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest

CHECKER = Path(__file__).with_name("check-notification-capabilities.sh")
CRITICAL = "com.apple.developer.usernotifications.critical-alerts"
FILTERING = "com.apple.developer.usernotifications.filtering"


class NotificationCapabilityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / "Synara.app"
        self.appex = self.app / "PlugIns" / "SynaraNotificationService.appex"
        self.appex.mkdir(parents=True)
        self.mock_bin = self.root / "bin"
        self.mock_bin.mkdir()
        mock = self.mock_bin / "codesign"
        mock.write_text('#!/usr/bin/env python3\nimport pathlib,sys\np=pathlib.Path(sys.argv[-1])/"signed.plist"\nif not p.exists(): sys.exit(1)\nsys.stdout.buffer.write(p.read_bytes())\n')
        mock.chmod(0o755)
        self.environment = dict(os.environ, PATH=f"{self.mock_bin}:{os.environ['PATH']}")
        self.configure(False, False)

    def configure(self, critical, filtering):
        flags = {"SynaraCriticalAlertsEnabled": "YES" if critical else "NO",
                 "SynaraNotificationFilteringEnabled": "YES" if filtering else "NO"}
        for bundle in [self.app, self.appex]:
            (bundle / "Info.plist").write_bytes(plistlib.dumps(flags))

    def sign(self, bundle, entitlements):
        (bundle / "signed.plist").write_bytes(plistlib.dumps(entitlements))

    def run_check(self):
        return subprocess.run([str(CHECKER), str(self.app)], env=self.environment,
                              capture_output=True, text=True)

    def test_default_build_needs_no_restricted_entitlements(self):
        self.assertEqual(self.run_check().returncode, 0)

    def test_critical_requires_both_signed_targets(self):
        self.configure(True, False)
        self.sign(self.app, {CRITICAL: True})
        self.sign(self.appex, {CRITICAL: True})
        self.assertEqual(self.run_check().returncode, 0)
        self.sign(self.appex, {})
        self.assertNotEqual(self.run_check().returncode, 0)

    def test_filtering_requires_extension_signature(self):
        self.configure(False, True)
        self.sign(self.appex, {FILTERING: False})
        self.assertNotEqual(self.run_check().returncode, 0)
        self.sign(self.appex, {FILTERING: True})
        self.assertEqual(self.run_check().returncode, 0)

    def test_combined_build_requires_both_capabilities(self):
        self.configure(True, True)
        self.sign(self.app, {CRITICAL: True})
        self.sign(self.appex, {CRITICAL: True, FILTERING: True})
        self.assertEqual(self.run_check().returncode, 0)
        self.sign(self.app, {})
        self.assertNotEqual(self.run_check().returncode, 0)

    def test_mismatched_build_flags_fail(self):
        (self.app / "Info.plist").write_bytes(plistlib.dumps({"SynaraCriticalAlertsEnabled": "YES"}))
        self.assertNotEqual(self.run_check().returncode, 0)

    def test_entitlements_require_boolean_values(self):
        self.configure(True, False)
        self.sign(self.appex, {CRITICAL: True})
        for wrong_type in ["true", "YES", 1]:
            with self.subTest(value=wrong_type):
                self.sign(self.app, {CRITICAL: wrong_type})
                self.assertNotEqual(self.run_check().returncode, 0)

    def test_unresolved_build_flags_fail(self):
        (self.app / "Info.plist").write_bytes(plistlib.dumps({"SynaraCriticalAlertsEnabled": "$(SYNARA_CRITICAL_ALERTS_ENABLED)"}))
        self.assertNotEqual(self.run_check().returncode, 0)


if __name__ == "__main__":
    unittest.main()
