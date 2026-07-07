# Goal Completion Formatting Rule

When acting as Principal Implementation Engineer and Independent Verification Recorder, every goal completion MUST be accompanied by a structured evidence report that can be used for a full forensic audit. 

**Do NOT provide only a narrative summary.**

You MUST use the exact format below for every goal completion:

```markdown
# TASK REPORT

## Goal

<exact goal executed>

---

## Executive Summary

* What was attempted
* What was completed
* What remains incomplete
* Overall status:
  * COMPLETE
  * PARTIAL
  * FAILED

---

## Files Modified

List every file modified.
For each file include:
* reason for modification
* major functions added/removed/changed

---

## Architectural Impact

Describe:
* runtime changes
* ownership changes
* dependency changes
* event flow changes
* API changes
* persistence changes
* replay changes

Explicitly state whether this modification affects: Perception, Memory, Reasoning, Decision, Execution, API, Runtime.

---

## Runtime Verification

Trace the runtime path affected by this goal.
State exactly what is VERIFIED versus ASSUMED.

---

## Commands Executed

Provide every command run.

---

## Test Results

Provide raw results (Passed/Failed/Ignored counts, and list any failing tests).
Do not summarize only. Include actual counts.

---

## Build Verification

State:
* Did project compile?
* Did workspace compile?
* Were warnings present?
* Were errors present?
Include relevant compiler output snippets.

---

## Runtime Evidence

If runtime was executed, provide logs and evidence that the modified path actually executed.
Do not only state that it worked. Show proof.

---

## Regression Analysis

Check whether this goal could impact: replay, persistence, state reconstruction, event bus, API compatibility, UI compatibility.
List any potential regressions.

---

## Known Remaining Gaps

List all gaps discovered during implementation in the format:
GAP-ID
Severity
Description
Impact
Recommended Fix

---

## Production Readiness Impact

State whether this goal moves Chronos toward: Prototype, Alpha, Beta, Production.
Explain why.

---

## Confidence Assessment

Provide confidence levels for Implementation, Build, Runtime, and Architectural confidence.
State what evidence supports each confidence estimate.

---

## Recommended Next Goal

Provide exactly one next goal. Choose the highest-leverage remaining architectural objective and explain why it should be next.

---

# AUDIT MODE RULES

* Never claim VERIFIED unless supported by code, tests, or runtime evidence.
* Distinguish VERIFIED, INFERRED, and ASSUMED.
* Include raw evidence whenever possible.
* If tests were not run, explicitly state: TESTS NOT RUN.
* If runtime was not executed, explicitly state: RUNTIME NOT VERIFIED.
* If compilation was not performed, explicitly state: BUILD NOT VERIFIED.
```
