#!/usr/bin/env python3
"""Run the agent-git-landing suites and write their results as a docs page.

Runs the contract proofs (`landing_tests` in modality-common) and the
end-to-end run (`test.sh`), then writes what each check reported to
docs/tutorials/agent-git-landing-results.md and the site's copy. The page
says the commit and date it ran at. It is written whether the checks pass
or fail; the exit status is non-zero when any failed.

Run from anywhere: python3 tests/network/agent-git-landing/report.py
"""

import datetime
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PAGE = pathlib.Path("docs/tutorials/agent-git-landing-results.md")
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def run(cmd, cwd):
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True)
    return proc.returncode, ANSI.sub("", proc.stdout + proc.stderr)


def proofs():
    """(name, passed) for each landing_tests case."""
    _, out = run(
        ["cargo", "test", "-p", "modality-common", "--features", "model-governance",
         "--lib", "landing_tests"],
        ROOT / "rust",
    )
    cases = []
    for m in re.finditer(r"^test model_governance::landing_tests::(\w+) \.\.\. (\w+)$", out, re.M):
        cases.append((m.group(1).replace("_", " ").capitalize(), m.group(2) == "ok"))
    if not cases:
        raise SystemExit(f"no landing_tests results found:\n{out[-3000:]}")
    return cases


def end_to_end():
    """(section, check, passed) for each step test.sh reports."""
    # test.sh uses the workspace's modal binary; build it at this commit first.
    code, out = run(["cargo", "build", "--package", "modal"], ROOT / "rust")
    if code:
        raise SystemExit(f"modal did not build:\n{out[-3000:]}")
    _, out = run(["./test.sh"], HERE)
    steps, section = [], "Setup"
    for line in out.splitlines():
        text = line.strip()
        if line and not line.startswith(" ") and text.endswith("..."):
            section = text.rstrip(".")
        elif line.startswith(" ") and (text.startswith("✓ ") or text.startswith("✗ ")):
            # Checks are indented; the run's own summary line is not.
            steps.append((section, text[2:], text.startswith("✓")))
    if not steps:
        raise SystemExit(f"no end-to-end results found:\n{out[-3000:]}")
    return steps


def page(commit, dirty, date, proof_cases, steps):
    total = len(proof_cases) + len(steps)
    failed = sum(not ok for _, ok in proof_cases) + sum(not ok for _, _, ok in steps)
    mark = lambda ok: "Pass" if ok else "**Fail**"
    at = f"`{commit}`" + (" with uncommitted changes" if dirty else "")
    lead = (
        f"All {total} checks passed at commit {at} on {date}."
        if not failed
        else f"{failed} of {total} checks failed at commit {at} on {date}."
    )
    lines = [
        "---",
        "sidebar_position: 7",
        "title: Agent on a Git Repo — Test Results",
        "---",
        "",
        "# Agent on a Git Repo: Test Results",
        "",
        lead,
        "",
        "These are the results of the two suites behind",
        "[An Agent on a Git Repo](agent-git-landing.md). The page is written by",
        "`tests/network/agent-git-landing/report.py` from a real run; it is not",
        "edited by hand.",
        "",
        "| Suite | What it checks | Checks | Passed | Failed |",
        "| --- | --- | --- | --- | --- |",
        f"| Contract proofs | The rules and witness model under theory `v2` and `v3`, commit by commit | "
        f"{len(proof_cases)} | {sum(ok for _, ok in proof_cases)} | {sum(not ok for _, ok in proof_cases)} |",
        f"| End-to-end run | A local hub, a bare origin, five keys, and `modal-git` doing each step | "
        f"{len(steps)} | {sum(ok for _, _, ok in steps)} | {sum(not ok for _, _, ok in steps)} |",
        "",
        "## Contract proofs",
        "",
        "`landing_tests` in `rust/modality-common` reads the example's own",
        "`model/default.modality` and `rules.txt`. Each case runs under theory",
        "`v2` and again under `v3`.",
        "",
        "| Case | Result |",
        "| --- | --- |",
    ]
    lines += [f"| {name} | {mark(ok)} |" for name, ok in proof_cases]
    lines += [
        "",
        "## End-to-end run",
        "",
        "`tests/network/agent-git-landing/test.sh`. A check that something is",
        "refused passes when it is refused. For the forged land, the run also",
        "checks that the hub's refusal names the rule it breaks.",
        "",
        "| Stage | Check | Result |",
        "| --- | --- | --- |",
    ]
    lines += [f"| {section} | {check} | {mark(ok)} |" for section, check, ok in steps]
    lines += [
        "",
        "## Run it yourself",
        "",
        "```bash",
        "# Both suites, and this page",
        "python3 tests/network/agent-git-landing/report.py",
        "",
        "# Or each on its own",
        "(cd rust && cargo test -p modality-common --features model-governance --lib landing_tests)",
        "tests/network/agent-git-landing/test.sh",
        "```",
        "",
    ]
    return "\n".join(lines), failed


def main():
    commit = run(["git", "rev-parse", "--short=12", "HEAD"], ROOT)[1].strip()
    dirty = bool(run(["git", "status", "--porcelain", "--", "rust", "examples/agent-git-landing",
                      "tests/network/agent-git-landing/test.sh"], ROOT)[1].strip())
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    text, failed = page(commit, dirty, date, proofs(), end_to_end())
    for path in (ROOT / PAGE, ROOT / "sites/www.modality.org" / PAGE):
        path.write_text(text)
        print(f"wrote {path.relative_to(ROOT)}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
