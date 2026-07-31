let meta = #{
    name: "matrix-pr-babysit",
    description: "One-shot babysit for a Matrix Rust integration PR: tip-merge, CI awareness, undraft+merge only if full-vertical criteria met",
    when_to_use: "When a single open PR into feature/matrix-rust-sdk-full-replacement needs tip-merge or green merge",
    phases: [
        #{ title: "Inspect", detail: "PR state, checks, mergeability" },
        #{ title: "Repair", detail: "tip-merge or CI fix if needed" },
        #{ title: "Gate", detail: "full-vertical acceptance before merge" },
    ],
};

// args.pr (required): PR number as string or number-like string
// args.allow_merge (optional): "true" to permit undraft+merge when green; default true
// args.hold_cutover (optional): "true" to never merge (e.g. #240 timeline HOLD)

let pr = if args == () { () } else { args.pr };
if pr == () {
    pause("verification", "Pass args.pr — GitHub PR number targeting feature/matrix-rust-sdk-full-replacement.");
}

let allow_merge = true;
if args != () && args.allow_merge != () {
    if args.allow_merge == "false" || args.allow_merge == false {
        allow_merge = false;
    }
}
let hold_cutover = false;
if args != () && args.hold_cutover != () {
    if args.hold_cutover == "true" || args.hold_cutover == true {
        hold_cutover = true;
    }
}

let base = "feature/matrix-rust-sdk-full-replacement";

phase("Inspect");
log("Inspect PR " + pr.to_string());

let insp = "";
insp += "Inspect GitHub PR #" + pr.to_string() + " into " + base + ".\n";
insp += "Use gh and git tools. Report:\n";
insp += "- draft/open/merged, head SHA, mergeable, behind tip?\n";
insp += "- required checks status (Quality, package, Validate, Synapse proofs as applicable)\n";
insp += "- whether PR claims full-vertical completion or is HOLD/incomplete\n";
insp += "- files touch risk (lib.rs, product.rs, dual_backend, secrets)\n";
insp += "Do not merge yet.\n";

let r1 = agent(insp, #{
    label: "inspect",
    capability_mode: "execute",
    agent_type: "general-purpose",
});

if r1 == () || !r1.success {
    complete(#{ ok: false, pr: pr, stage: "inspect", error: "inspect failed" });
}

phase("Repair");
log("Repair PR " + pr.to_string() + " if needed");

let repair = "";
repair += "For PR #" + pr.to_string() + " against " + base + ":\n";
repair += "1. If behind tip or CONFLICTING: tip-merge origin/" + base + " into the PR branch using a /private/tmp/synara-codex-* worktree; resolve unions carefully; push.\n";
repair += "2. If CI red with an obvious fix (prettier, clippy, trivial): fix, push, do not force unrelated refactors.\n";
repair += "3. If green and only waiting: report wait.\n";
repair += "4. Never merge #39/main. Never reopen plateau acceptance.\n";
repair += "5. hold_cutover=" + hold_cutover.to_string() + " — if true, never undraft/merge.\n";
repair += "Prior inspect:\n";
repair += json_encode(r1.output) + "\n";

let r2 = agent(repair, #{
    label: "repair",
    capability_mode: "all",
    agent_type: "general-purpose",
});

phase("Gate");
log("Full-vertical merge gate for PR " + pr.to_string());

let gate = "";
gate += "Decide whether PR #" + pr.to_string() + " may undraft and merge into " + base + ".\n";
gate += "allow_merge=" + allow_merge.to_string() + " hold_cutover=" + hold_cutover.to_string() + "\n\n";
gate += "Merge ONLY if ALL of:\n";
gate += "1. hold_cutover is false\n";
gate += "2. allow_merge is true\n";
gate += "3. Required CI green on exact head (Quality + Desktop package + required Validate/Synapse as applicable)\n";
gate += "4. MERGEABLE / not draft-required policy conflict\n";
gate += "5. Full-vertical for claimed scope: native sole owner + physical JS deletion for that capability (or honest docs-only PR)\n";
gate += "6. No dual_backend, no plateau residual acceptance, no secrets in IPC returns\n";
gate += "7. For V-TIMELINE/#240: only if presenter selected AND RoomTimeline deleted AND proof claimed — else FAIL merge\n\n";
gate += "If merge allowed: undraft, gh pr merge --merge, report new tip.\n";
gate += "If not: leave draft, list blockers only.\n";
gate += "Inspect/repair context:\n";
gate += json_encode(r1.output) + "\n";
gate += json_encode(r2.output) + "\n";

let r3 = agent(gate, #{
    label: "gate",
    capability_mode: "all",
    agent_type: "general-purpose",
});

let report = "";
report += "# matrix-pr-babysit #" + pr.to_string() + "\n\n";
report += "## Inspect\n" + json_encode(r1.output) + "\n\n";
report += "## Repair\n";
if r2 != () && r2.success { report += json_encode(r2.output); } else { report += "repair incomplete"; }
report += "\n\n## Gate\n";
if r3 != () && r3.success { report += json_encode(r3.output); } else { report += "gate incomplete"; }
report += "\n";

let path = write_scratch_file("matrix-pr-babysit-report.md", report);
complete(#{
    ok: true,
    pr: pr,
    hold_cutover: hold_cutover,
    allow_merge: allow_merge,
    report_path: path,
    gate_ok: r3 != () && r3.success,
});
