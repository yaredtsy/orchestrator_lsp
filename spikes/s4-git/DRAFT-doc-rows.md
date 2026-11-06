# Draft rows for the docs you maintain (not applied)

Written 2026-11-06 from the S4 provisional-tier work. Nothing here has been put into `01-decision-log.md`, `01-review-checklist.md` or `05-spikes/`.

## 1. Decision log: new row (next free number after VS-D73)

| VS-D74 | **[you] A middle tier between a silent match and a question: the provisional match.** **Provisional when:** the top candidate passes a **provisional floor** (below the silent threshold), has a **clear margin** over the second, and **no ask-only rule applies** (twins, minimum token count); anything else asks, as before. **Shown as:** the docs sit on the candidate with an **"Unconfirmed link"** label; actions **Confirm**, **pick another candidate**, **Detach** (back to Missing). **State:** provisional state lives **only in SQLite**; `node.toml` keeps the old anchor until **Confirm**, which writes the new anchor (the shared action). The ladder is deterministic, so teammates compute the same provisional match. **No ID is written into code for a provisional match**; inject only on Confirm. **MCP:** `node_context` returns `link: "unconfirmed"` for these docs. **One grouped notice per sync** with "Confirm all". **Labels never expire or auto-approve.** **Floor: not set yet** (see S4 findings: no floor reaches 95% on the labelled set). | refines VS-D44, VS-D73 | 05-spikes/04 |

## 2. Decision log: amendment to VS-D73

Add to the end of the row: "**Ask-only rules** (they cap a match at *ask*, never silent, never provisional): a lost body with an **identical twin anywhere** in the before-state, and a body under the **minimum token count**. Measured in S4: with the twin rule on, silently wrong twin cases go from 95 of 191 (train) to 0, at a cost of 2 silent matches (446 to 444)."

## 3. Checklist

No new row needed. VS-O10 can be marked answered by the S4 findings once you accept them (rule: silent only at Edit score >= 0.99; see findings).

## 4. S4 findings: provisional tier (paste into `05-spikes/04-git-and-matching.md`)

**Code was restructured first, with no behaviour change** (the fixtures and 82 lines of tuning output are identical to the saved baseline), so the tiers are now one pure function (`ladder/policy.rs`, with tests) and the tuning harness calls the same `resolve` the ladder uses. See `spikes/s4-git/spike-match/README.md`.

**Measurement.** Cases that stay a *soft* question under the silent rule (the score or the lead is not good enough; not a twin, not a short body): 4,086 train, 3,729 held-out. For each, the top candidate by Edit score.

Top-1 accuracy per Edit-score band, **lead over the second >= 0.10** (train / held-out):
| band | train | held-out |
|---|---|---|
| 0.50-0.55 | 95% (94 cases) | 99% (154) |
| 0.55-0.60 | 98% (327) | 99% (424) |
| 0.60-0.65 | 100% (425) | 98% (484) |
| 0.65-0.70 | 93% (238) | 92% (202) |
| 0.75-0.80 | 91% (335) | 97% (301) |
| 0.80-0.85 | 71% (160) | 79% (205) |
| 0.85-0.90 | 77% (452) | 94% (371) |
| 0.90-0.95 | 70% (699) | 90% (527) |
| 0.95-1.00 | 68% (379) | 100% (277) |

**No floor reaches 95% top-1 accuracy.** Accuracy is **not monotone in the score**: a floor includes every band above it, and the high bands are the polluted ones. What a floor can reach (train / held-out):
| target | floor / lead | links | accuracy |
|---|---|---|---|
| 99%, 95% | none reaches it | | |
| 90% | 0.50 / 0.30 | 2,183 / 2,397 | 91.0% / 98.4% |
| 85% | 0.45 / 0.20 | 2,681 / 2,757 | 85.6% / 96.8% |
| 80% | 0.30 / 0.10 | 3,231 / 3,115 | 82.3% / 93.9% |

**Why the high bands are wrong** (Edit >= 0.80, lead >= 0.10): edits of 0-5%: **654 of 654 right** (train), 658 of 658 (held-out). Edits of 10-20%: 87% / 97% right. Edits of **35%+: 9 of 367 right** (0 of 86 held-out): the true successor only scores ~0.6, so a candidate at >= 0.80 is a lookalike. The simulation plants one on purpose (a copy of the closest sibling in the same file). In the 41 + 11 *deleted* cases any link is wrong.

**What would move from ask to provisional** (of 4,354 asked, train / of 4,134, held-out):
| floor / lead | moved | right | wrong (of which for a deleted function) |
|---|---|---|---|
| 0.50 / 0.30 | 2,183 / 2,397 | 1,987 / 2,359 | 196 (42) / 38 (14) |
| 0.45 / 0.20 | 2,681 / 2,757 | 2,296 / 2,670 | 385 (54) / 87 (27) |
| 0.30 / 0.10 | 3,231 / 3,115 | 2,659 / 2,924 | 572 (80) / 191 (44) |
Silent matches are unchanged (444 train, 423 held-out).

On the refactor fixtures, with floor 0.30 / lead 0.10 the one miss (rename a function **and** rewrite its body) becomes a **provisional, right** link; the twin methods still ask (`[Twin]`).

**Open (your call):** the floor. The 95% target is out of reach on this set; 90% gives a floor of 0.50 with a lead of 0.30. A near-twin rule (like the twin rule, but for functions that are *almost* identical to the lost one) would likely remove most of the high-band errors; not tried.

## 5. Sweep list for the other docs (hand to the other Claude)

Home technical docs hold the full rule; summary docs only link to **VS-D74**.
- `02-technology/05-identity-and-archive.md` §4 (the ladder): add the provisional tier after step 5, with the conditions and the "never in code until Confirm" rule. §5 (states and diagram): add the **provisional** state (docs on the candidate, "Unconfirmed link"; Confirm / pick another / Detach), and that it is SQLite-only.
- `02-technology/04-storage-and-git.md`: `node.toml` keeps the old anchor until Confirm (the shared action writes the new one); provisional state is **not** in `.vnoc/`.
- `02-technology/03-frontend-webview.md`: the label and its three actions on the doc panel; the grouped notice with **Confirm all**; labels never expire.
- `00-requirements/03-functional-requirements.md`: a new FR (next free number) for the tier; `00-requirements/02-user-journey.md`: where a rename-and-edit now lands (an unconfirmed link, not "Find it...").
- Summary docs (README, `04-roadmap/01-mvp-vs-stable.md`): one line linking VS-D74. MCP `node_context` returning `link: "unconfirmed"` has no doc yet (phase 4): leave it in the decision row.
- Do **not** write an accuracy figure or floor value anywhere until the floor is decided.
