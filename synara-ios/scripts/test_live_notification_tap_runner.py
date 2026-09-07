"""Completion/cleanup regressions; no network, credentials, Xcode or simulator."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "notification_tap_runner", Path(__file__).with_name("run-live-notification-tap.py")
)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class NotificationTapRunnerCompletionTests(unittest.TestCase):
    complete_trace = "\n".join(
        "synara_notification_tap phase=" + phase + "-" + stage
        for phase in ("warm", "cold") for stage in ("ready", "confirmed")
    )

    def run_fixture(self, trace, failed_logout=None):
        cleanup = []
        sessions = iter([("reader-token", "@reader:example.org"),
                         ("writer-token", "@writer:example.org")])

        def urlopen(request, timeout):
            self.assertEqual(timeout, 30)
            path = request.full_url.split("/_matrix/client/v3/")[1]
            token = request.get_header("Authorization", "").removeprefix("Bearer ")
            if path == "login":
                token, user = next(sessions)
                result = {"access_token": token, "user_id": user}
            elif path == "createRoom":
                result = {"room_id": "!fixture:example.org"}
            elif "/send/" in path:
                result = {"event_id": "$" + path.rsplit("/", 1)[1]}
            elif path.endswith("/leave") or path == "logout":
                cleanup.append(("logout" if path == "logout" else "leave", token))
                if path == "logout" and token == failed_logout:
                    raise OSError("fixture logout failure")
                result = {}
            else:
                result = {}
            return io.BytesIO(json.dumps(result).encode())

        environment = {
            "SYNARA_LIVE_HOMESERVER": "https://matrix.example.org",
            "SYNARA_LIVE_USERNAME": "reader", "SYNARA_LIVE_PASSWORD": "fixture-only",
            "SYNARA_LIVE_SECOND_USERNAME": "writer", "SYNARA_LIVE_SECOND_PASSWORD": "fixture-only",
            "SYNARA_QA_SIMULATOR_ID": "fixture-simulator",
            "SYNARA_QA_DEVELOPMENT_TEAM": "fixture-team",
            "SYNARA_QA_DERIVED_DATA": "/tmp/fixture-derived",
        }
        with tempfile.TemporaryDirectory() as directory, \
             patch.dict(os.environ, environment, clear=True), \
             patch.object(runner.os, "umask"), \
             patch.object(runner.tempfile, "mkdtemp", return_value=directory), \
             patch.object(runner.urllib.request, "urlopen", side_effect=urlopen), \
             patch.object(runner.subprocess, "Popen") as popen, \
             patch.object(runner.subprocess, "run") as push, \
             contextlib.redirect_stdout(io.StringIO()):
            popen.return_value.stdout = io.StringIO(trace)
            popen.return_value.wait.return_value = 0
            result = runner.main()
        return result, cleanup, push.call_count

    def assert_all_sessions_cleaned(self, cleanup):
        self.assertEqual(cleanup, [
            ("leave", "writer-token"), ("logout", "writer-token"),
            ("leave", "reader-token"), ("logout", "reader-token"),
        ])

    def test_successful_ui_with_failed_logout_fails_and_cleans_remaining_session(self):
        result, cleanup, pushes = self.run_fixture(self.complete_trace, failed_logout="writer-token")
        self.assertNotEqual(result, 0)
        self.assertEqual(pushes, 2)
        self.assert_all_sessions_cleaned(cleanup)

    def test_both_confirmed_routes_and_successful_cleanup_pass(self):
        result, cleanup, pushes = self.run_fixture(self.complete_trace)
        self.assertEqual(result, 0)
        self.assertEqual(pushes, 2)
        self.assert_all_sessions_cleaned(cleanup)

    def test_skipped_xctest_cannot_report_success(self):
        result, cleanup, pushes = self.run_fixture("Test skipped: fixture unavailable\n")
        self.assertNotEqual(result, 0)
        self.assertEqual(pushes, 0)
        self.assert_all_sessions_cleaned(cleanup)

    def test_delivered_notifications_without_ui_readback_fail(self):
        trace = self.complete_trace.replace("synara_notification_tap phase=cold-confirmed", "")
        result, cleanup, pushes = self.run_fixture(trace)
        self.assertNotEqual(result, 0)
        self.assertEqual(pushes, 2)
        self.assert_all_sessions_cleaned(cleanup)


if __name__ == "__main__":
    unittest.main()
