import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";

const setup = `
import importlib.util,hashlib,datetime,copy
spec=importlib.util.spec_from_file_location('checker','scripts/check-macos-provisioning-profile.py')
checker=importlib.util.module_from_spec(spec);spec.loader.exec_module(checker)
certificate=b'fixture-public-certificate'
fingerprint=hashlib.sha1(certificate).hexdigest().upper()
profile={'Entitlements':{checker.APP_IDENTIFIER:'TEAM.com.whylandcreative.synara.desktop',checker.TEAM_IDENTIFIER:'TEAM',checker.TIME_SENSITIVE:True},'TeamIdentifier':['TEAM'],'Platform':['OSX'],'ProvisionsAllDevices':True,'DeveloperCertificates':[certificate],'ExpirationDate':datetime.datetime(2027,1,1)}
now=datetime.datetime(2026,10,4,tzinfo=datetime.timezone.utc)
def validate(p): return checker.validate_profile(p,'TEAM','com.whylandcreative.synara.desktop',{fingerprint},now)
`;

function python(code) {
  const result = spawnSync("python3", ["-B", "-c", setup + code], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
}

test("accepts a matching unexpired macOS notification profile", () => {
  python("assert validate(profile)[checker.TIME_SENSITIVE] is True");
});

for (const [label, mutation] of [
  ["another bundle", "p['Entitlements'][checker.APP_IDENTIFIER]='TEAM.other'"],
  ["another team", "p['TeamIdentifier']=['OTHER']"],
  ["iOS distribution", "p['Platform']=['iOS']"],
  ["device-restricted distribution", "p['ProvisionsAllDevices']=False"],
  [
    "debugging access",
    "p['Entitlements']['com.apple.security.get-task-allow']=True",
  ],
  ["expired", "p['ExpirationDate']=datetime.datetime(2026,1,1)"],
  [
    "a different signing certificate",
    "p['DeveloperCertificates']=[b'other-public-certificate']",
  ],
]) {
  test(`rejects a profile with ${label}`, () => {
    python(
      `p=copy.deepcopy(profile)\n${mutation}\ntry:\n validate(p)\nexcept ValueError:\n pass\nelse:\n raise AssertionError('invalid profile accepted')`
    );
  });
}

test("requires a true boolean capability, not strings or integers", () => {
  python(`for value in [False,None,'true','YES',1]:
 p=copy.deepcopy(profile);p['Entitlements'][checker.TIME_SENSITIVE]=value
 try: validate(p)
 except ValueError: pass
 else: raise AssertionError('untyped entitlement accepted')`);
});

test("selects only the exact Developer ID identity or fingerprint", () => {
  python(`name='Developer ID Application: Example (TEAM)'
output=f'  1) {fingerprint} "{name}"\\n  2) {fingerprint} "Apple Development: Example"'
assert checker.identity_fingerprints(output,name)=={fingerprint}
assert checker.identity_fingerprints(output,fingerprint)=={fingerprint}
assert checker.identity_fingerprints(output,'Example')==set()`);
});

test("CLI binds generated entitlements and rejects tampered embedded profiles", () => {
  python(`import tempfile,pathlib,plistlib,subprocess,os,stat
with tempfile.TemporaryDirectory() as temporary:
 root=pathlib.Path(temporary);bin=root/'bin';bin.mkdir()
 source=root/'profile.provisionprofile';source.write_bytes(plistlib.dumps(profile))
 entitlements=root/'source.plist';entitlements.write_bytes(plistlib.dumps({checker.TIME_SENSITIVE:True}))
 generated=root/'generated.plist';generated.write_text('prior');generated.chmod(0o644)
 security=bin/'security'
 security.write_text('#!/usr/bin/env python3\\nimport os,sys\\nfrom pathlib import Path\\nif sys.argv[1]=="cms":sys.stdout.buffer.write(Path(os.environ["FIXTURE_PROFILE"]).read_bytes())\\nelse:print(os.environ["FIXTURE_IDENTITIES"])\\n');security.chmod(0o700)
 env=dict(os.environ,PATH=str(bin)+os.pathsep+os.environ['PATH'],FIXTURE_PROFILE=str(source),FIXTURE_IDENTITIES=f' 1) {fingerprint} "Developer ID Application: Example (TEAM)"')
 command=['python3','scripts/check-macos-provisioning-profile.py',str(source),'--team','TEAM','--identity','Developer ID Application: Example (TEAM)']
 result=subprocess.run(command+['--entitlements',str(entitlements),'--entitlements-output',str(generated)],capture_output=True,env=env)
 assert result.returncode==0,result.stderr
 claims=plistlib.loads(generated.read_bytes())
 assert claims[checker.APP_IDENTIFIER]=='TEAM.com.whylandcreative.synara.desktop'
 assert claims[checker.TEAM_IDENTIFIER]=='TEAM'
 assert claims[checker.TIME_SENSITIVE] is True
 assert stat.S_IMODE(generated.stat().st_mode)==0o600
 app=root/'Synara.app';(app/'Contents').mkdir(parents=True)
 (app/'Contents/embedded.provisionprofile').write_bytes(b'tampered-profile')
 result=subprocess.run(command+['--app',str(app)],capture_output=True,env=env)
 assert result.returncode==1
 assert b'does not embed the validated' in result.stderr
 assert b'tampered-profile' not in result.stderr`);
});

test("CLI rejects an actual signer outside the profile even when identity names match", () => {
  python(`import tempfile,pathlib,plistlib,subprocess,os
with tempfile.TemporaryDirectory() as temporary:
 root=pathlib.Path(temporary);bin=root/'bin';bin.mkdir()
 source=root/'profile.provisionprofile';source.write_bytes(plistlib.dumps(profile))
 unauthorized=b'renewed-public-certificate'
 unauthorized_fingerprint=hashlib.sha1(unauthorized).hexdigest().upper()
 leaf=root/'leaf.der';leaf.write_bytes(unauthorized)
 signed=root/'signed.plist';signed.write_bytes(plistlib.dumps(profile['Entitlements']))
 app=root/'Synara.app';(app/'Contents').mkdir(parents=True)
 (app/'Contents/embedded.provisionprofile').write_bytes(source.read_bytes())
 (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':'com.whylandcreative.synara.desktop'}))
 security=bin/'security'
 security.write_text('#!/usr/bin/env python3\\nimport os,sys\\nfrom pathlib import Path\\nif sys.argv[1]=="cms":sys.stdout.buffer.write(Path(os.environ["FIXTURE_PROFILE"]).read_bytes())\\nelse:print(os.environ["FIXTURE_IDENTITIES"])\\n');security.chmod(0o700)
 codesign=bin/'codesign'
 codesign.write_text('#!/usr/bin/env python3\\nimport os,sys\\nfrom pathlib import Path\\nif "--entitlements" in sys.argv:sys.stdout.buffer.write(Path(os.environ["FIXTURE_SIGNED"]).read_bytes())\\nelse:\\n prefix=next(arg.split("=",1)[1] for arg in sys.argv if arg.startswith("--extract-certificates="))\\n Path(prefix+"0").write_bytes(Path(os.environ["FIXTURE_LEAF"]).read_bytes())\\n');codesign.chmod(0o700)
 identity='Developer ID Application: Example (TEAM)'
 env=dict(os.environ,PATH=str(bin)+os.pathsep+os.environ['PATH'],FIXTURE_PROFILE=str(source),FIXTURE_SIGNED=str(signed),FIXTURE_LEAF=str(leaf),FIXTURE_IDENTITIES=f' 1) {fingerprint} "{identity}"\\n 2) {unauthorized_fingerprint} "{identity}"')
 assert checker.identity_fingerprints(env['FIXTURE_IDENTITIES'],identity)=={fingerprint,unauthorized_fingerprint}
 checker.validate_profile(profile,'TEAM','com.whylandcreative.synara.desktop',{fingerprint,unauthorized_fingerprint},now)
 command=['python3','-B','scripts/check-macos-provisioning-profile.py',str(source),'--team','TEAM','--identity',identity,'--app',str(app)]
 result=subprocess.run(command,capture_output=True,env=env)
 assert result.returncode==1,result.stdout
 assert b"does not authorize the final app's signing certificate" in result.stderr
 assert unauthorized not in result.stderr
 assert unauthorized_fingerprint.encode() not in result.stderr
 leaf.write_bytes(certificate)
 result=subprocess.run(command,capture_output=True,env=env)
 assert result.returncode==0,result.stderr`);
});

test("release and manual signed lanes embed the validated profile before signing", () => {
  for (const file of ["release.yml", "macos-signed-build.yml"]) {
    const workflow = readFileSync(`.github/workflows/${file}`, "utf8");
    assert.match(
      workflow,
      /MACOS_PROVISIONING_PROFILE_BASE64: \$\{\{ secrets\.MACOS_PROVISIONING_PROFILE_BASE64 \}\}/
    );
    assert.match(
      workflow,
      /files: \{ "embedded\.provisionprofile": process\.env\.SYNARA_MACOS_PROVISIONING_PROFILE \}/
    );
    assert.match(
      workflow,
      /entitlements: process\.env\.SYNARA_MACOS_SIGNING_ENTITLEMENTS/
    );
    assert.ok(
      workflow.indexOf(
        "Validate and prepare Developer ID notification profile"
      ) < workflow.indexOf("--bundles app,dmg") ||
        file === "macos-signed-build.yml"
    );
    assert.match(workflow, /--app /);
  }
  const smoke = readFileSync(
    ".github/workflows/desktop-package-smoke.yml",
    "utf8"
  );
  assert.match(smoke, /"entitlements":"Entitlements\.adhoc\.plist"/);
  assert.match(
    readFileSync("src-tauri/Entitlements.plist", "utf8"),
    /time-sensitive/
  );
});

function workflowStep(workflow, name) {
  const step = workflow
    .split(/^      - name: /m)
    .slice(1)
    .find((entry) => entry.startsWith(`${name}\n`));
  assert.ok(step, `Missing workflow step: ${name}`);
  return step;
}

function assertBuildNotarizationCredentials(workflow, name) {
  const build = workflowStep(workflow, name);
  assert.match(build, /npm run tauri build --/);
  const env = build.match(/^        env:\n((?:          .*\n)+)/m)?.[1] ?? "";
  assert.match(env, /^          APPLE_ID: \$\{\{ secrets\.APPLE_ID \}\}$/m);
  assert.match(
    env,
    /^          APPLE_PASSWORD: \$\{\{ secrets\.APPLE_APP_SPECIFIC_PASSWORD \}\}$/m
  );
  assert.match(
    env,
    /^          APPLE_TEAM_ID: \$\{\{ secrets\.APPLE_TEAM_ID \}\}$/m
  );
}

for (const [file, name] of [
  ["release.yml", "Build macOS universal release packages"],
  ["macos-signed-build.yml", "Build signed macOS DMG"],
]) {
  test(`${file} provides notarization credentials to the actual Tauri build`, () => {
    const workflow = readFileSync(`.github/workflows/${file}`, "utf8");
    assertBuildNotarizationCredentials(workflow, name);
    const build = workflowStep(workflow, name);
    for (const key of ["APPLE_ID", "APPLE_PASSWORD"]) {
      const strippedBuild = build.replace(
        new RegExp(`^          ${key}: .*\\n`, "m"),
        ""
      );
      assert.notEqual(strippedBuild, build);
      const missingBuildCredential = workflow.replace(build, strippedBuild);
      // Validation and later notarytool steps still contain the secrets. Those
      // decoys must not satisfy the build-step credential contract.
      assert.match(missingBuildCredential, /secrets\.APPLE_ID/);
      assert.match(
        missingBuildCredential,
        /secrets\.APPLE_APP_SPECIFIC_PASSWORD/
      );
      assert.throws(() =>
        assertBuildNotarizationCredentials(missingBuildCredential, name)
      );
    }
  });
}

test("manual signed app verification reads back its profile before mandatory Gatekeeper assessment", () => {
  const workflow = readFileSync(
    ".github/workflows/macos-signed-build.yml",
    "utf8"
  );
  const verify = workflowStep(workflow, "Verify macOS app signature");
  const signature = verify.indexOf("codesign --verify --deep --strict");
  const profile = verify.indexOf(
    "python3 scripts/check-macos-provisioning-profile.py"
  );
  const assessment = verify.indexOf("spctl --assess --type execute");
  assert.ok(signature >= 0 && profile > signature && assessment > profile);
  assert.doesNotMatch(verify, /continue-on-error|\|\| true/);
});
