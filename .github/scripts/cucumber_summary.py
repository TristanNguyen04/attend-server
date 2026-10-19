#!/usr/bin/env python3
"""Print a Markdown summary of a Cucumber JSON report (for the GitHub job summary).

Usage: cucumber_summary.py <cucumber.json>

A scenario passed when all its steps passed, failed when any step failed, was undefined, was
ambiguous or is pending, and is skipped otherwise. Exits 0 even when scenarios failed: the test
step is what fails the job; this only reports.
"""
import json
import sys
from pathlib import Path

FAILED = {"failed", "undefined", "ambiguous", "pending"}
ICON = {"passed": "✅", "failed": "❌", "skipped": "⏭️"}


def scenario_status(steps):
    statuses = {step.get("result", {}).get("status") for step in steps}
    if statuses & FAILED:
        return "failed"
    if "skipped" in statuses:
        return "skipped"
    return "passed"


def cell(text):
    return str(text).replace("|", "\\|").replace("\n", " ")


def main(path):
    report = Path(path)
    print("## Cucumber scenarios\n")
    if not report.is_file():
        print(f"No report found at `{path}`: the Cucumber run did not finish.")
        return

    rows = []
    for feature in json.loads(report.read_text()):
        for element in feature.get("elements", []):
            steps = element.get("steps", [])
            errors = [
                step["result"]["error_message"]
                for step in steps
                if step.get("result", {}).get("error_message")
            ]
            rows.append(
                (feature["name"], element["name"], scenario_status(steps), errors)
            )

    counts = {status: sum(1 for row in rows if row[2] == status) for status in ICON}
    print(
        f"**Scenarios:** {len(rows)} total, {counts['passed']} passed, "
        f"{counts['failed']} failed, {counts['skipped']} skipped\n"
    )
    print("| Status | Feature | Scenario |")
    print("|---|---|---|")
    for feature, scenario, status, _ in rows:
        print(f"| {ICON[status]} {status} | {cell(feature)} | {cell(scenario)} |")

    for feature, scenario, status, errors in rows:
        if status == "failed" and errors:
            print(f"\n<details><summary>{cell(feature)}: {cell(scenario)}</summary>\n")
            print("```")
            print("\n".join(errors))
            print("```\n\n</details>")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
