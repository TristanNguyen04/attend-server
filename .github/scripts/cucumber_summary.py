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


def example_label(element):
    """What tells the rows of a Scenario Outline apart: the action step with its example values.

    Every row of an Examples table runs as its own scenario with the same title, so without this
    the table would list the same name several times. Empty for a plain scenario.
    """
    if element.get("keyword") != "Scenario Outline":
        return ""
    for step in element.get("steps", []):
        if step.get("keyword", "").strip() == "When":
            return step["name"]
    return f"line {element.get('line')}"


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
        # Cucumber JSON lists a scenario, then (when the feature has a Background) a separate
        # "background" element that carries the same name. It belongs to that scenario: its steps
        # count towards the scenario's status, and it is not a scenario of its own.
        scenarios = []
        for element in feature.get("elements", []):
            if element.get("type") == "background" and scenarios:
                scenarios[-1]["steps"] += element.get("steps", [])
            else:
                scenarios.append(
                    {
                        "name": element["name"],
                        "steps": list(element.get("steps", [])),
                        "example": example_label(element),
                    }
                )
        for scenario in scenarios:
            steps = scenario["steps"]
            errors = [
                step["result"]["error_message"]
                for step in steps
                if step.get("result", {}).get("error_message")
            ]
            rows.append(
                (
                    feature["name"],
                    scenario["name"],
                    scenario["example"],
                    scenario_status(steps),
                    errors,
                )
            )

    counts = {status: sum(1 for row in rows if row[3] == status) for status in ICON}
    print(
        f"**Scenarios:** {len(rows)} total, {counts['passed']} passed, "
        f"{counts['failed']} failed, {counts['skipped']} skipped\n"
    )
    print("| Status | Feature | Scenario | Example |")
    print("|---|---|---|---|")
    for feature, scenario, example, status, _ in rows:
        print(
            f"| {ICON[status]} {status} | {cell(feature)} | {cell(scenario)} | {cell(example)} |"
        )

    for feature, scenario, example, status, errors in rows:
        if status == "failed" and errors:
            title = f"{cell(feature)}: {cell(scenario)}" + (f" ({cell(example)})" if example else "")
            print(f"\n<details><summary>{title}</summary>\n")
            print("```")
            print("\n".join(errors))
            print("```\n\n</details>")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
