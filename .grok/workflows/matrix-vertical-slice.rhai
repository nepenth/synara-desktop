let meta = #{
    name: "matrix-vertical-slice",
    description: "Implement one Matrix Rust full-vertical residual slice: inventory, product wire + JS deletion, scoped verify, draft PR",
    when_to_use: "When advancing the residual queue with a named slice (e.g. V-SEND.5, V-ROOMS.2c) under full-vertical policy",
    phases: [
        #{ title: "Inventory", detail: "map current JS owner and Rust foundations" },
        #{ title: "Implement", detail: "UI to Tauri to live matrix-sdk + physical deletion" },
        #{ title: "Verify", detail: "scoped tests and full-vertical acceptance" },
        #{ title: "Publish", detail: "push branch and open draft PR" },
    ],
};

// args.slice_id (required): residual ID string, e.g. "V-SEND.5"
// args.branch (optional): git branch name; default matrix-rust/<slug>
// args.worktree (optional): absolute worktree path under /private/tmp
// args.goal (optional): one-line done-when override

let slice_id = if args == () { () } else { args.slice_id };
if slice_id == () {
    pause("verification", "Pass args.slice_id — residual ID from d0-residual-completion.md (e.g. V-SEND.5).");
}

let branch = if args == () || args.branch == () {
    "matrix-rust/slice-" + slice_id
} else {
    args.branch
};
let worktree = if args == () || args.worktree == () {
    "/private/tmp/synara-codex-" + slice_id
} else {
    args.worktree
};
let goal = if args == () || args.goal == () {
    "Full vertical for " + slice_id + ": native sole owner via UI to Tauri IPC to live matrix-sdk, plus physical deletion of superseded matrix-js-sdk product owners for this capability only."
} else {
    args.goal
};

let base = "feature/matrix-rust-sdk-full-replacement";
let policy = "docs/matrix-rust-sdk/full-vertical-policy.md";
let residual = "docs/matrix-rust-sdk/d0-residual-completion.md";
let handoff = "docs/matrix-rust-sdk/SESSION-HANDOFF.md";

phase("Inventory");
log("Inventory for slice " + slice_id);

let inv_prompt = "";
inv_prompt += "You are inventorying a Matrix Rust full-replacement vertical slice.\n";
inv_prompt += "Integration branch: " + base + "\n";
inv_prompt += "Slice ID: " + slice_id + "\n";
inv_prompt += "Goal: " + goal + "\n\n";
inv_prompt += "Read with tools (read_file, grep, git):\n";
inv_prompt += "- " + residual + "\n";
inv_prompt += "- " + policy + "\n";
inv_prompt += "- " + handoff + "\n";
inv_prompt += "- origin/" + base + " tip\n\n";
inv_prompt += "Inventory must use tools, not memory. Return a markdown report covering:\n";
inv_prompt += "1. Current product JS owners (files/hooks) for this capability\n";
inv_prompt += "2. Existing Rust host modules / L1 parts to reuse\n";
inv_prompt += "3. Tauri commands/permissions likely needed\n";
inv_prompt += "4. Physical deletion targets (JS files/imports/tests)\n";
inv_prompt += "5. Explicit non-goals (other residual IDs only)\n";
inv_prompt += "6. Acceptance checklist for full-vertical done\n";
inv_prompt += "Do not implement code in this phase.\n";

let inv = agent(inv_prompt, #{
    label: "inventory",
    capability_mode: "read-only",
    agent_type: "explore",
});

if inv == () || !inv.success {
    complete(#{
        ok: false,
        slice_id: slice_id,
        stage: "inventory",
        error: "inventory agent failed",
    });
}

phase("Implement");
log("Implement full vertical " + slice_id);

let impl_prompt = "";
impl_prompt += "You implement one full-vertical Matrix Rust residual slice.\n";
impl_prompt += "Slice ID: " + slice_id + "\n";
impl_prompt += "Branch: " + branch + "\n";
impl_prompt += "Worktree: " + worktree + "\n";
impl_prompt += "Base: origin/" + base + "\n";
impl_prompt += "Goal: " + goal + "\n\n";
impl_prompt += "HARD POLICY:\n";
impl_prompt += "- No dual_backend / runtime selector\n";
impl_prompt += "- No dogfood minimum / plateau residual acceptance\n";
impl_prompt += "- Physical deletion of superseded JS owners for THIS capability\n";
impl_prompt += "- No tokens/keys/recovery secrets/ciphertext in IPC returns or logs\n";
impl_prompt += "- Never merge PR #39 to main; never revive plateau #221\n";
impl_prompt += "- Work only under /private/tmp/synara-codex-* worktrees\n\n";
impl_prompt += "Setup (execute):\n";
impl_prompt += "1. git fetch origin " + base + "\n";
impl_prompt += "2. Ensure worktree at " + worktree + " on branch " + branch + " from origin/" + base + "\n";
impl_prompt += "3. Implement UI to Tauri to live matrix-sdk for this slice\n";
impl_prompt += "4. Delete superseded matrix-js-sdk product owners for this capability\n";
impl_prompt += "5. Wire permissions/schemas/lib registration as needed\n";
impl_prompt += "6. Add/update docs/matrix-rust-sdk note for this slice if helpful\n";
impl_prompt += "7. Do NOT open PR yet (Publish phase does that)\n";
impl_prompt += "8. Do NOT merge anything\n\n";
impl_prompt += "Inventory context from prior phase (may be incomplete; re-verify with tools):\n";
impl_prompt += json_encode(inv.output) + "\n";

let impl_r = agent(impl_prompt, #{
    label: "implement",
    capability_mode: "all",
    agent_type: "general-purpose",
});

if impl_r == () || !impl_r.success {
    complete(#{
        ok: false,
        slice_id: slice_id,
        stage: "implement",
        error: "implement agent failed",
        inventory: inv.output,
    });
}

phase("Verify");
log("Verify full-vertical acceptance for " + slice_id);

let ver_prompt = "";
ver_prompt += "Adversarially verify the worktree " + worktree + " for slice " + slice_id + ".\n";
ver_prompt += "Use tools: git status/diff, read_file, grep, run scoped tests if available.\n\n";
ver_prompt += "Fail closed if any of these are missing evidence:\n";
ver_prompt += "1. Native product path owns the capability (no live js client for happy path)\n";
ver_prompt += "2. Superseded JS owners for THIS capability deleted or clearly fail-closed on native session\n";
ver_prompt += "3. No dual_backend / no plateau language as acceptance\n";
ver_prompt += "4. No secrets in IPC returns\n";
ver_prompt += "5. Scoped fmt/tests/typecheck evidence for touched modules (or document why not run)\n\n";
ver_prompt += "Return markdown: verdict Confirmed|Not confirmed|Failed, evidence bullets, residual blockers.\n";

let ver = agent(ver_prompt, #{
    label: "verify",
    capability_mode: "execute",
    agent_type: "general-purpose",
});

if ver == () || !ver.success {
    complete(#{
        ok: false,
        slice_id: slice_id,
        stage: "verify",
        error: "verify agent failed",
        implement: impl_r.output,
    });
}

phase("Publish");
log("Publish draft PR for " + slice_id);

let pub_prompt = "";
pub_prompt += "Publish the slice branch as a DRAFT PR.\n";
pub_prompt += "Worktree: " + worktree + "\n";
pub_prompt += "Branch: " + branch + "\n";
pub_prompt += "Base: " + base + "\n";
pub_prompt += "Slice: " + slice_id + "\n\n";
pub_prompt += "Steps:\n";
pub_prompt += "1. In worktree: commit any remaining changes if needed (complete sentences)\n";
pub_prompt += "2. Push origin " + branch + "\n";
pub_prompt += "3. Open DRAFT PR with gh if none exists; base " + base + "\n";
pub_prompt += "4. PR body: summary, deletion delta if measured, residual named IDs, validation run\n";
pub_prompt += "5. Do NOT undraft, do NOT merge\n\n";
pub_prompt += "Verify notes:\n";
pub_prompt += json_encode(ver.output) + "\n";
pub_prompt += "Return the PR URL and head SHA.\n";

let pub_r = agent(pub_prompt, #{
    label: "publish",
    capability_mode: "all",
    agent_type: "general-purpose",
});

let report = "";
report += "# matrix-vertical-slice report\n\n";
report += "- slice_id: " + slice_id + "\n";
report += "- branch: " + branch + "\n";
report += "- worktree: " + worktree + "\n\n";
report += "## Inventory\n";
report += json_encode(inv.output) + "\n\n";
report += "## Implement\n";
report += json_encode(impl_r.output) + "\n\n";
report += "## Verify\n";
report += json_encode(ver.output) + "\n\n";
report += "## Publish\n";
if pub_r != () && pub_r.success {
    report += json_encode(pub_r.output) + "\n";
} else {
    report += "publish agent failed or incomplete\n";
}

let path = write_scratch_file("matrix-vertical-slice-report.md", report);
complete(#{
    ok: true,
    slice_id: slice_id,
    branch: branch,
    worktree: worktree,
    report_path: path,
    publish_ok: pub_r != () && pub_r.success,
});
