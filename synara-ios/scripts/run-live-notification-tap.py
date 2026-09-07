#!/usr/bin/env python3
"""Authorized, opt-in simulator notification tap proof with disposable fixtures.

Requires SYNARA_LIVE_* credentials for two test accounts, SYNARA_QA_SIMULATOR_ID,
SYNARA_QA_DEVELOPMENT_TEAM, and SYNARA_QA_DERIVED_DATA. No credential is logged.
Writes private logs/results under a fresh temporary directory. This proves
simulator notification taps; it does not prove physical APNs/NSE execution.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.parse
import urllib.request
import uuid


def main():
    os.umask(0o077)
    env = os.environ.copy()
    required = ["SYNARA_LIVE_HOMESERVER", "SYNARA_LIVE_USERNAME", "SYNARA_LIVE_PASSWORD",
                "SYNARA_LIVE_SECOND_USERNAME", "SYNARA_LIVE_SECOND_PASSWORD",
                "SYNARA_QA_SIMULATOR_ID", "SYNARA_QA_DEVELOPMENT_TEAM", "SYNARA_QA_DERIVED_DATA"]
    if any(not env.get(key) for key in required):
        raise SystemExit("Missing explicit test-account or dedicated simulator configuration")
    base = env["SYNARA_LIVE_HOMESERVER"].rstrip("/") + "/_matrix/client/v3/"
    proof = Path(tempfile.mkdtemp(prefix="synara-notification-tap-"))
    tokens = []
    room = None

    def request(method, path, payload=None, token=None):
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = "Bearer " + token
        data = None if payload is None else json.dumps(payload).encode()
        with urllib.request.urlopen(urllib.request.Request(base + path, data=data, headers=headers, method=method), timeout=30) as response:
            return json.load(response)

    def login(prefix):
        value = request("POST", "login", {"type": "m.login.password", "identifier": {
            "type": "m.id.user", "user": env[prefix + "USERNAME"]},
            "password": env[prefix + "PASSWORD"], "initial_device_display_name": "Synara notification tap fixture"})
        tokens.append(value["access_token"])
        return value

    try:
        reader = login("SYNARA_LIVE_")
        writer = login("SYNARA_LIVE_SECOND_")
        nonce = uuid.uuid4().hex[:8]
        name = "Notification " + nonce
        created = request("POST", "createRoom", {"name": name, "preset": "private_chat",
            "visibility": "private", "invite": [writer["user_id"]]}, reader["access_token"])
        room = urllib.parse.quote(created["room_id"], safe="")
        request("POST", "join/" + room, {}, writer["access_token"])
        events = [request("PUT", "rooms/" + room + "/send/m.room.message/" + uuid.uuid4().hex,
            {"msgtype": "m.text", "body": "Notification context fixture " + str(index)}, writer["access_token"])["event_id"] for index in range(3)]
        fixture = {"SYNARA_LIVE_NOTIFICATION_TAP_SMOKE": "1", "SYNARA_LIVE_NOTIFICATION_ROOM_ID": created["room_id"],
            "SYNARA_LIVE_NOTIFICATION_ROOM_NAME": name, "SYNARA_LIVE_NOTIFICATION_EVENT_ID": events[1],
            "SYNARA_LIVE_NOTIFICATION_PREVIOUS_EVENT_ID": events[0], "SYNARA_LIVE_NOTIFICATION_FOLLOWING_EVENT_ID": events[2],
            "SYNARA_LIVE_NOTIFICATION_BANNER_TITLE": "Context proof " + nonce}
        for key, value in {**{key: value for key, value in env.items() if key.startswith("SYNARA_LIVE_")}, **fixture}.items():
            env["TEST_RUNNER_" + key] = value
        payload = proof / "notification.apns"
        payload.write_text(json.dumps({"aps": {"alert": {"title": fixture["SYNARA_LIVE_NOTIFICATION_BANNER_TITLE"],
            "body": "Open the notified message"}, "sound": "default"}, "room_id": created["room_id"], "event_id": events[1]}))
        args = ["xcodebuild", "test", "-project", "Synara.xcodeproj", "-scheme", "Synara", "-destination",
            "platform=iOS Simulator,id=" + env["SYNARA_QA_SIMULATOR_ID"], "-derivedDataPath", env["SYNARA_QA_DERIVED_DATA"],
            "-resultBundlePath", str(proof / "result.xcresult"),
            "-only-testing:SynaraUITests/SynaraUITests/testLiveNotificationTapContextWhenConfigured",
            "-parallel-testing-enabled", "NO", "GENERATE_INFOPLIST_FILE=YES", "CODE_SIGNING_ALLOWED=YES", "CODE_SIGNING_REQUIRED=YES",
            "DEVELOPMENT_TEAM=" + env["SYNARA_QA_DEVELOPMENT_TEAM"], "CODE_SIGN_IDENTITY=Apple Development", "ARCHS=arm64", "ONLY_ACTIVE_ARCH=YES"]
        sent = set()
        with (proof / "xcode.log").open("w") as log:
            process = subprocess.Popen(args, cwd=Path(__file__).resolve().parents[1], env=env,
                                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
            for line in process.stdout:
                log.write(line)
                log.flush()
                for phase in ("warm", "cold"):
                    if "synara_notification_tap phase=" + phase + "-ready" in line and phase not in sent:
                        subprocess.run(["xcrun", "simctl", "push", env["SYNARA_QA_SIMULATOR_ID"],
                            "com.whylandcreative.synara", str(payload)], check=True, stdout=log, stderr=log)
                        sent.add(phase)
            code = process.wait()
        print("Notification tap proof exit", code, "phases delivered", len(sent), "private results", proof)
        return code
    finally:
        cleanup_ok = True
        for token in reversed(tokens):
            if room:
                try:
                    request("POST", "rooms/" + room + "/leave", {}, token)
                except Exception:
                    cleanup_ok = False
            try:
                request("POST", "logout", {}, token)
            except Exception:
                cleanup_ok = False
        print("Notification fixture cleanup", "complete" if cleanup_ok else "needs attention")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception:
        # urllib/SDK exceptions may include request URLs; keep console private.
        raise SystemExit("Notification fixture failed; inspect private local evidence")
