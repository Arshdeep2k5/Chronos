# Chronos Pilot Demo Execution Script

This document details the step-by-step demonstration sequence designed to showcase the features of Chronos Pilot.

## 1. Demo Checklist & Preparation
* Clean the local directory: Remove any dynamic data by running `rm ~/.config/chronos/chronos.db`
* Ensure local VSCode can be executed from shell commands (verify `code --version` executes cleanly).
* Have a mock PDF asset named `ML_Assignment.pdf` available in the `~/Downloads` directory. Ensure it has "ML Assignment due July 15" written within the text.

---

## 2. Interactive Scene Flow

### Scene 1: Zero-Configuration Commitment Discovery
* **Visual**: Show the blank Chronos HUD. Open a local file browser displaying the `~/Downloads` folder containing `ML_Assignment.pdf`.
* **Action**: Drag the PDF file into a mock directory watched by the Chronos Filesystem Watcher.
* **Explanation**: *"Our developer has just downloaded their ML coursework assignment. Instead of relying on manual task list configuration, watch Chronos Pilot identify this asset."*
* **Outcome**: The HUD displays a notification stating: **`Commitment Discovered: ML Assignment | Type: ASSIGNMENT | Deadline: July 15 | Conf: 92%`**.

---

### Scene 2: Proactive Focus Tracking & Web Research
* **Visual**: Open a browser tab. Navigate to Google and type: `"RAG chunking strategy embeddings example"`. Click through StackOverflow and documentation pages.
* **Action**: Let the active window linger on documentation for 15 seconds.
* **Explanation**: *"As the user researches approaches, Chronos’s Browser Telemetry Connector tracks active focus duration, tab navigation, and search queries, grouping them dynamically into semantic sessions under the ML Assignment project."*
* **Outcome**: Bring the HUD to the foreground; show the real-time graph growth and show the **`Research Session: RAG Chunking`** updated on the HUD.

---

### Scene 3: Time Jump & Risk Forecasting
* **Visual**: Fast-forward time manually by sending a simulated datetime update event to the local SQLite DB, simulating 3 days of no project progress.
* **Action**: The HUD turns deep Amber, then triggers a red flashing alert.
* **Explanation**: *"Three days pass with no file edits or project workspace events. Chronos Pilot detects this gap. Our risk forecasting dashboard recalculates the user’s trajectory."*
* **Outcome**: The HUD displays the warning card: **`Risk Score: 0.81 (HIGH) | Completeness Chance: 41% | Simulated Failure Date: July 12`**. Show the consequence simulator: **`Postponing 24h drops completion chance to 28% (-13% drop)`**.

---

### Scene 4: Recovery Planning Synthesis
* **Visual**: Point the cursor to the "Generate Recovery Plan" action link displayed inside the Risk card on the HUD.
* **Action**: Click **"Generate Recovery Plan"**.
* **Explanation**: *"Rather than just reminding the user that they are in danger of failing, Chronos generates a prioritized daily catch-up schedule."*
* **Outcome**: A step-by-step interactive checklist renders on the HUD displaying task splits:
  * `[Today] - Complete literature review & parse inputs (3.5h)`
  * `[Tomorrow] - Train local classifier & run baseline (4.0h)`
  * `[Wednesday] - Finalize report writing & export PDFs (2.5h)`

---

### Scene 5: One-Click Restoration & "Why You Stopped"
* **Visual**: Hover cursor over the primary **"Start Working"** button.
* **Action**: Click **"Start Working"**.
* **Explanation**: *"Let's remove the cognitive friction of restarting. Clicking 'Start Working' reconstructs the physical workspace layout."*
* **Outcome**:
  1. VSCode automatically launches, loads the target notebook, and positions the cursor at line 42.
  2. The browser automatically launches, opening the tab cluster containing the Stripe documentation files.
  3. The HUD slide-out opens adjacent to the IDE, displaying the **`Why You Stopped Card`** detailing exactly which search terms and code blockers were active when momentum stalled.

---

### Scene 6: Autonomous Research Brief
* **Visual**: Zoom in on the HUD context card detailing the **"While You Were Away"** panel.
* **Action**: Click **"While You Were Away"**.
* **Explanation**: *"While the developer was struggling with momentum blockages, Chronos worked in the background. It identified the core blockage and searched arXiv for solutions."*
* **Outcome**: The HUD displays two pre-summarized research briefs directly answering the developer’s last Google search queries.
