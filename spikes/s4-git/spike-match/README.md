# spike-match: the matching ladder (spike S4)

How a documented node is found again after a refactor, and how sure we are. Spike code, but kept tidy so the ideas
can be improved without it turning into a mess. Decisions live in `plan_vs/01-decisions` (VS-D44, D72, D73 and the
provisional tier); this file is only about the code.

## The idea in one screen

```
 before (cache)  ┐                                   ┌─ tier 1  Silent       linked, no label
                 ├─ steps 1-3 ──► what is left ──►   ├─ tier 2  Provisional  linked, "Unconfirmed link"
 after (git)     ┘  (path, file hash, git -M)        ├─ tier 3  Ask          "Find it…", ranked, with reasons
                                  │                  └─         Nothing      Missing
                          steps 4-5 hand their candidates to  ladder/policy.rs
```

## Layout

| Path | What it is |
|---|---|
| `src/model.rs` | `Def`, `Snapshot`, `Node`: the data the ladder works on |
| `src/driver.rs` | client for the S2 driver (`vnoc/defs` gives each def's tokens and parameters) |
| `src/git/` | `GitBackend` trait (`mod.rs`) and the `git` binary implementation (`cli.rs`); a gix one will sit next to it |
| `src/similarity.rs` | the two body scores (Jaccard, edit distance) and the context features |
| `src/ladder/config.rs` | **every knob**, grouped: `SilentRule`, `ProvisionalRule`, `AskRules` |
| `src/ladder/policy.rs` | **the tier rules**: pure functions, with the table-driven tests |
| `src/ladder/function.rs`, `nodes.rs` | steps 1-3 for functions, files and folders |
| `src/ladder/matching.rs` | who is still unmatched after steps 1-3 (the twin bookkeeping) |
| `src/ladder/rank.rs` | scoring and ordering candidates |
| `src/ladder/outcome.rs` | what the product is told: `Found`, `Provisional`, `Ask { reasons }`, `Missing` |
| `src/cli.rs` | flags both binaries share |
| `src/bin/fixtures/` | six refactors (+ extras) on a small repo, one git branch each |
| `src/bin/tune/` | the labelled-set tuning: `dataset`, `simulate`, `counts`, `silent`, `provisional`, `report` |
| `gen_labelled.py` | builds the labelled set from real repos (mutated functions, truth known) |

## Where do I change X?

| I want to… | Change |
|---|---|
| move a threshold, or turn the provisional tier on or off | `Config` (`config.rs`), or a flag: `--silent=T,M` `--provisional=F,M` `--ask=MIN,on\|off,FLOOR` |
| add a rule that forces a question (like twins) | one `if` in `policy::ask_only_reasons` + a variant in `AskReason` + a test row |
| change what makes a link silent or provisional | `policy::is_silent` / `is_provisional` + a test row |
| add a new score or context feature | `similarity.rs` (`Features`), then use it in `policy.rs` |
| add or change a ladder step | `function.rs` (steps 1-3) or `policy::step4` |
| change what the user is told | `outcome.rs` |
| add a git backend | implement `GitBackend` (`git/mod.rs`) |
| add a fixture | one `Case` in `src/bin/fixtures/cases.rs`: the edit, plus the nodes whose truth moved |

## One rule: the tuning harness never re-implements the product rule

`tune` calls `ladder::resolve`, the same function the ladder uses (`finish`). If the rule changes in `policy.rs`,
the tuning changes with it. Do not copy decision logic into a binary.

## Running

```sh
cargo test                                        # policy tiers, matching, twins (pure, fast)
cargo run --release --bin fixtures                # the refactor fixtures, ladder end to end (needs the S2 driver built)
python3 gen_labelled.py <repo> data/<name>.json   # once per repo
S4_SCRATCH=<dir with httpx requests flask> S4_LIGHTRAG=<repo> cargo run --release --bin tune
```

The S2 driver must be built first: `spikes/s2-python-driver/spike-driver-python` (`cargo build --release`).

## Known limits (so nobody is surprised later)

- `score_pool` computes edit distance for only 16 candidates per lost function (12 by Jaccard, 4 by name).
- The simulated "distractors" are other functions of the same repo copied under new names, plus the 3 closest siblings
  in the same file. That is a hard setup on purpose; real commits are usually easier.
- Rename detection (step 3) is git's (`-M`), only for files. There is no function-level rename detection except steps 4-5.
