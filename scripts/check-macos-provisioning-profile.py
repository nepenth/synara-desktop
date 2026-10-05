#!/usr/bin/env python3
"""Validate Developer ID notification provisioning without logging profile contents."""
import argparse
import datetime
import hashlib
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import tempfile

TIME_SENSITIVE = "com.apple.developer.usernotifications.time-sensitive"
APP_IDENTIFIER = "com.apple.application-identifier"
TEAM_IDENTIFIER = "com.apple.developer.team-identifier"


def validate_profile(profile, team, bundle_id, identity_fingerprints, now=None):
    entitlements = profile.get("Entitlements", {})
    if entitlements.get(APP_IDENTIFIER) != f"{team}.{bundle_id}":
        raise ValueError("Developer ID profile targets a different application identifier.")
    if team not in profile.get("TeamIdentifier", []):
        raise ValueError("Developer ID profile belongs to a different team.")
    if "OSX" not in profile.get("Platform", []):
        raise ValueError("A macOS Developer ID provisioning profile is required.")
    if profile.get("ProvisionsAllDevices") is not True:
        raise ValueError("A Developer ID profile for direct distribution to all devices is required.")
    if entitlements.get("com.apple.security.get-task-allow", False) is not False:
        raise ValueError("A distribution profile without debugging access is required.")
    if entitlements.get(TIME_SENSITIVE) is not True:
        raise ValueError("Developer ID profile must authorize Time Sensitive notifications.")
    expires = profile.get("ExpirationDate")
    if not isinstance(expires, datetime.datetime):
        raise ValueError("Developer ID profile has no valid expiration date.")
    expires = expires.replace(tzinfo=datetime.timezone.utc) if expires.tzinfo is None else expires
    if expires <= (now or datetime.datetime.now(datetime.timezone.utc)):
        raise ValueError("Developer ID profile has expired.")
    allowed = {
        hashlib.sha1(certificate).hexdigest().upper()
        for certificate in profile.get("DeveloperCertificates", [])
        if isinstance(certificate, bytes)
    }
    if not allowed.intersection(identity_fingerprints):
        raise ValueError("Developer ID profile does not authorize the selected signing certificate.")
    return entitlements


def identity_fingerprints(output, identity):
    matches = re.findall(r'^\s*\d+\)\s+([A-Fa-f0-9]{40})\s+"([^"]+)"', output, re.MULTILINE)
    return {
        fingerprint.upper()
        for fingerprint, name in matches
        if name.startswith("Developer ID Application:")
        and (identity == name or identity.upper() == fingerprint.upper())
    }


def run(*arguments):
    result = subprocess.run(arguments, capture_output=True, check=False)
    if result.returncode:
        raise ValueError(f"Unable to inspect Apple signing data using {arguments[0]}.")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", type=Path)
    parser.add_argument("--team", required=True)
    parser.add_argument("--identity", required=True)
    parser.add_argument("--bundle-id", default="com.whylandcreative.synara.desktop")
    parser.add_argument("--entitlements", type=Path)
    parser.add_argument("--entitlements-output", type=Path)
    parser.add_argument("--app", type=Path)
    args = parser.parse_args()
    profile = plistlib.loads(run("security", "cms", "-D", "-i", str(args.profile)))
    identities = identity_fingerprints(run("security", "find-identity", "-v", "-p", "codesigning").decode(), args.identity)
    allowed = validate_profile(profile, args.team, args.bundle_id, identities)
    if args.entitlements_output:
        if not args.entitlements:
            raise ValueError("An entitlements source is required to generate signing entitlements.")
        claimed = plistlib.loads(args.entitlements.read_bytes())
        if claimed.get(TIME_SENSITIVE) is not True:
            raise ValueError("Signing entitlements must claim Time Sensitive notifications as a boolean.")
        claimed[APP_IDENTIFIER] = f"{args.team}.{args.bundle_id}"
        claimed[TEAM_IDENTIFIER] = args.team
        for key, value in claimed.items():
            if key.startswith("com.apple.developer.") and allowed.get(key) != value:
                raise ValueError("The selected profile does not authorize a requested developer entitlement.")
        with os.fdopen(os.open(args.entitlements_output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "wb") as output:
            os.fchmod(output.fileno(), 0o600)
            plistlib.dump(claimed, output)
    if args.app:
        embedded = args.app / "Contents/embedded.provisionprofile"
        if not embedded.is_file() or embedded.read_bytes() != args.profile.read_bytes():
            raise ValueError("The app does not embed the validated Developer ID profile.")
        info = plistlib.loads((args.app / "Contents/Info.plist").read_bytes())
        if info.get("CFBundleIdentifier") != args.bundle_id:
            raise ValueError("The final app bundle identifier does not match its notification profile.")
        signed = plistlib.loads(run("codesign", "-d", "--entitlements", ":-", str(args.app)))
        if signed.get(TIME_SENSITIVE) is not True or signed.get(APP_IDENTIFIER) != f"{args.team}.{args.bundle_id}" or signed.get(TEAM_IDENTIFIER) != args.team:
            raise ValueError("The final app signature is missing its validated notification entitlements.")
        with tempfile.TemporaryDirectory(prefix="synara-signer-") as temporary:
            prefix = str(Path(temporary) / "certificate")
            run("codesign", "-d", f"--extract-certificates={prefix}", str(args.app))
            signer = hashlib.sha1(Path(prefix + "0").read_bytes()).hexdigest().upper()
            if signer not in identities:
                raise ValueError("The final app uses a different Developer ID signing certificate.")
    print("Developer ID notification profile and selected signing identity are compatible.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, plistlib.InvalidFileException) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
