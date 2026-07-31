let meta = #{
    name: "matrix-residual-audit",
    description: "Docs-only residual/import truth-up for Matrix Rust full-replacement tracking",
    when_to_use: "After product merges, or when CONTINUATION/PROGRESS/residual ledger drift from tip",
    phases: [
        #{ title: "Measure", detail: "import counts and open PRs" },
        #{ title: "Write", detail: "update tracking docs and draft PR" },
    ],
};

// args optional; always bases on origin/feature/matrix-rust-sdk-full-replacement

let base = "feature/matrix-rust-sdk-full-replacement";
let worktree = "/private/tmp/synara-codex-docs-residual-audit";
let branch = "matrix-rust/docs-residual-audit";

phase("Measure");
log("Measure tip residual truth");

let m = "";
m += "Measure Matrix Rust replacement tracking truth on origin/" + base + ".\n";
m += "Use git/gh tools only (read-only for product code).\n";
m += "Report:\n";
m += "1. Tip SHA and last few merge subjects\n";
m += "2. Production and total matrix-js-sdk importer file counts under synara/src\n";
m += "3. Open PRs into " + base + " with disposition (active/HOLD/docs)\n";
m += "4. Residual queue next IDs from docs/matrix-rust-sdk/d0-residual-completion.md\n";
m += "5. Stale claims in CONTINUATION/PROGRESS/SESSION-HANDOFF if any\n";
m += "Do not edit files in this phase.\n";

let r1 = agent(m, #{
    label: "measure",
    capability_mode: "execute",
    agent_type: "explore",
});

if r1 == () || !r1.success {
    complete(#{ ok: false, stage: "measure", error: "measure failed" });
}

phase("Write");
log("Write tracking docs PR");

let w = "";
w += "Docs-only truth-up for Matrix Rust full-replacement.\n";
w += "Base: origin/" + base + "\n";
w += "Branch: " + branch + "\n";
w += "Worktree: " + worktree + "\n\n";
w += "Rules:\n";
w += "- Edit ONLY docs/matrix-rust-sdk/*.md tracking files (CONTINUATION, PROGRESS, residual, SESSION-HANDOFF as needed)\n";
w += "- No product Rust/TS implementation changes\n";
w += "- Do not claim verticals done without native+deletion\n";
w += "- Note Grok hybrid ops: thin babysit loop + matrix-vertical-slice / matrix-pr-babysit workflows\n";
w += "- Push draft or ready PR to " + base + "; do not merge\n\n";
w += "Measurement context:\n";
w += json_encode(r1.output) + "\n";

let r2 = agent(w, #{
    label: "write-docs",
    capability_mode: "all",
    agent_type: "general-purpose",
});

let report = "# matrix-residual-audit\n\n## Measure\n";
report += json_encode(r1.output) + "\n\n## Write\n";
if r2 != () && r2.success {
    report += json_encode(r2.output);
} else {
    report += "write failed";
}
report += "\n";

let path = write_scratch_file("matrix-residual-audit-report.md", report);
complete(#{
    ok: r2 != () && r2.success,
    report_path: path,
    measure: r1.output,
});
