import test from "node:test";
import assert from "node:assert/strict";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checker = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../check-synara-nse-core-archive-exports.sh"
);
const expected = "_uniffi_synara_nse_core_fn_method_nsepreviewrequest_resolve";
function executable(path, source) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, source);
  chmodSync(path, 0o755);
}
function fixture(
  t,
  {
    body = `echo '${expected} T 0 0'`,
    version = "22.1.2",
    arches = "arm64",
    override = true,
    missing = false,
  } = {}
) {
  const root = mkdtempSync(join(tmpdir(), "synara-nse-symbols-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const bin = join(root, "bin");
  const sysroot = join(root, "selected-sysroot");
  executable(
    join(bin, "rustc"),
    `#!/usr/bin/env bash\nif [[ "$1" == "-vV" ]]; then printf 'host: aarch64-apple-darwin\\nrelease: 1.96.1\\nLLVM version: 22.1.2\\n'; else printf '%s\\n' '${sysroot}'; fi\n`
  );
  executable(
    join(bin, "lipo"),
    `#!/usr/bin/env bash\n[[ "$1" == "-archs" ]] || exit 91\necho '${arches}'\n`
  );
  const nm = override
    ? join(bin, "fixture-llvm-nm")
    : join(sysroot, "lib/rustlib/aarch64-apple-darwin/bin/llvm-nm");
  if (!missing)
    executable(
      nm,
      `#!/usr/bin/env bash\nif [[ "$1" == "--version" ]]; then echo 'LLVM version ${version}'; exit 0; fi\n[[ "$1" == --arch=* && "$2" == --extern-only && "$3" == --defined-only && "$4" == --format=posix && "$5" == --quiet ]] || exit 92\n${body}\n`
    );
  // PATH's ordinary nm must never substitute for the selected Rust decoder.
  executable(
    join(bin, "nm"),
    "#!/usr/bin/env bash\necho 'wrong ordinary nm selected' >&2\nexit 93\n"
  );
  const env = { ...process.env, PATH: `${bin}:${process.env.PATH}` };
  delete env.SYNARA_NSE_ARCHIVE_NM;
  if (override) env.SYNARA_NSE_ARCHIVE_NM = nm;
  return {
    root,
    env,
    run: (...contents) => {
      const archives = contents.map((content, i) => {
        const archive = join(root, `libsynara_nse_core-${i}.a`);
        if (content !== null) writeFileSync(archive, content);
        return archive;
      });
      return spawnSync("bash", [checker, ...archives], {
        encoding: "utf8",
        env,
      });
    },
  };
}

test("positive narrow NSE decoded symbols pass with matching selected Rust tool", (t) => {
  const f = fixture(t, { override: false });
  const result = f.run("archive");
  assert.equal(result.status, 0, result.stderr);
  assert.match(
    result.stdout,
    /architecture=arm64 symbols=1 required_nse_export=/
  );
});
test("explicit matching fixture override remains supported", (t) => {
  const result = fixture(t).run("archive");
  assert.equal(result.status, 0, result.stderr);
});
test("a full Core export after large decoded output still fails", (t) => {
  const f = fixture(t, {
    body: `echo '${expected} T 0 0'\nfor ((i=0;i<40000;i++)); do printf 'neutral_%s T 0 0\\n' "$i"; done\necho '_uniffi_synara_core_fn_login T 0 0'`,
  });
  const result = f.run("archive");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});
test("raw archive bytes cannot substitute for or contradict decoded symbol output", (t) => {
  const result = fixture(t).run(
    "data contains _uniffi_synara_core_fn_login but no such export"
  );
  assert.equal(result.status, 0, result.stderr);
});
test("wrong LLVM reader version fails before decoding", (t) => {
  const result = fixture(t, { version: "21.1.0" }).run("archive");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /LLVM version mismatch/);
});
test("decoder mismatch never falls back to clean-looking raw bytes", (t) => {
  const result = fixture(t, {
    body: "echo 'Unknown attribute kind (Producer LLVM22 Reader LLVM21)' >&2; exit 1",
  }).run("clean-looking bytes");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /symbol inspection failed/);
});
test("decoder diagnostics even with zero exit status fail closed", (t) => {
  const result = fixture(t, {
    body: `echo '${expected} T 0 0'; echo 'member decoding failed' >&2`,
  }).run("archive");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /symbol inspection failed/);
});
for (const [label, body] of [
  ["empty", "exit 0"],
  ["headers only", "echo 'member.o:'"],
  ["other symbols", "echo '_some_symbol T 0 0'"],
  ["positive name without a symbol row", `echo '${expected} '`],
  ["undefined positive name", `echo '${expected} U 0 0'`],
]) {
  test(`${label} decoded output fails positive export requirement`, (t) => {
    const result = fixture(t, { body }).run("archive");
    assert.equal(result.status, 1);
    assert.match(result.stderr, /no required decoded NSE export/);
  });
}
test("each universal archive architecture requires its own NSE positive readback", (t) => {
  const result = fixture(t, {
    arches: "arm64 x86_64",
    body: `if [[ "$1" == --arch=arm64 ]]; then echo '${expected} T 0 0'; else echo '_other T 0 0'; fi`,
  }).run("fat archive");
  assert.equal(result.status, 1);
  assert.match(result.stdout, /architecture=arm64/);
  assert.match(result.stderr, /x86_64/);
});
test("a forbidden export in the second archive fails the whole inspection", (t) => {
  const result = fixture(t, {
    body: `echo '${expected} T 0 0'; if [[ "$6" == *-1.a ]]; then echo '_uniffi_synara_core_fn_login T 0 0'; fi`,
  }).run("first", "second");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});
test("missing selected llvm component fails without installing or using PATH nm", (t) => {
  const result = fixture(t, { override: false, missing: true }).run("archive");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Missing selected Rust llvm-nm/);
});
test("empty architecture inventory fails closed", (t) => {
  const result = fixture(t, { arches: "" }).run("archive");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /architecture inventory is empty/);
});
test("missing and empty archives fail before decoder selection", (t) => {
  const f = fixture(t, { missing: true });
  assert.match(f.run(null).stderr, /archive is missing/);
  assert.match(f.run("").stderr, /archive is empty/);
});
test("no archives is an inspection failure", () => {
  const result = spawnSync("bash", [checker], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Usage:/);
});

const shippingChecker = resolve(
  dirname(checker),
  "../synara-ios/scripts/check-notification-service-archive.sh"
);
function shippingFixture(t, options) {
  const f = fixture(t, options);
  const app = join(f.root, "Synara.app");
  const extension = join(app, "PlugIns/SynaraNotificationService.appex");
  mkdirSync(extension, { recursive: true });
  writeFileSync(join(app, "Info.plist"), "fixture app metadata");
  writeFileSync(join(extension, "Info.plist"), "fixture");
  writeFileSync(join(extension, "NSE"), "fixture image");
  const bin = join(f.root, "bin");
  executable(
    join(bin, "plutil"),
    `#!/usr/bin/env bash
case "$1" in
  -lint) [[ -s "$2" ]] ;;
  -extract)
    case "$2" in
      CFBundleExecutable) echo NSE ;;
      SynaraCriticalAlertsEnabled|SynaraNotificationFilteringEnabled) echo NO ;;
      *) exit 94 ;;
    esac ;;
  *) exit 94 ;;
esac
`
  );
  executable(join(bin, "stat"), "#!/usr/bin/env bash\necho 2048\n");
  executable(
    join(bin, "file"),
    "#!/usr/bin/env bash\necho 'Mach-O 64-bit executable arm64'\n"
  );
  executable(
    join(bin, "otool"),
    "#!/usr/bin/env bash\necho '/usr/lib/libSystem.B.dylib'\n"
  );
  const diagnostics = join(f.root, "diagnostics");
  return {
    ...f,
    diagnostics,
    runShipping: () =>
      spawnSync("bash", [shippingChecker, app, diagnostics], {
        encoding: "utf8",
        env: f.env,
      }),
  };
}
test("shipping final MachO uses the correct decoder and records positive readback", (t) => {
  const f = shippingFixture(t);
  const result = f.runShipping();
  assert.equal(result.status, 0, result.stderr);
});
test("shipping stripped symbol output is recorded without standalone archive proof", async (t) => {
  const { readFileSync } = await import("node:fs");
  const f = shippingFixture(t, { body: "exit 0" });
  const result = f.runShipping();
  assert.equal(result.status, 0, result.stderr);
  const report = readFileSync(
    join(f.diagnostics, "notification_service_archive_report.txt"),
    "utf8"
  );
  assert.match(report, /symbols=0/);
  assert.match(report, /not archive isolation proof/);
});
test("shipping decoder errors cannot be compensated with raw byte absence", (t) => {
  const result = shippingFixture(t, {
    body: "echo 'decoder failed' >&2; exit 1",
  }).runShipping();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /symbol inspection failed/);
});
test("shipping forbidden decoded full Core exports fail", (t) => {
  const result = shippingFixture(t, {
    body: `echo '${expected} T 0 0'; echo '_uniffi_synara_core_fn_login T 0 0'`,
  }).runShipping();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});
