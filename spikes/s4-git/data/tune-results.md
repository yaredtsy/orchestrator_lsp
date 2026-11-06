# S4 tuning results

train = LightRAG + httpx (4884 cases), held-out = requests + flask (4711 cases). ask floor 0.30, twin rule on.

## Edit score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): 24 of 1440

| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |
|---|---|---|---|---|---|---|---|---|---|
| 0.99 | 0.00 | 10 | 444 | 3053 | 887 | 0 | 154 | 191 | **0** |
| 0.99 | 0.10 | 10 | 434 | 3063 | 887 | 0 | 154 | 191 | **0** |
| 0.99 | 0.05 | 10 | 434 | 3063 | 887 | 0 | 154 | 191 | **0** |
| 0.99 | 0.15 | 10 | 430 | 3067 | 887 | 0 | 154 | 191 | **0** |
| 0.99 | 0.30 | 10 | 426 | 3071 | 887 | 0 | 154 | 191 | **0** |

## Jaccard score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): 212 of 1440

| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |
|---|---|---|---|---|---|---|---|---|---|
| 0.91 | 0.05 | 10 | 526 | 1401 | 487 | 1322 | 97 | 191 | **0** |
| 0.92 | 0.05 | 10 | 522 | 1405 | 487 | 1322 | 97 | 191 | **0** |
| 0.92 | 0.00 | 10 | 522 | 1405 | 487 | 1322 | 97 | 191 | **0** |
| 0.91 | 0.10 | 10 | 510 | 1417 | 487 | 1322 | 97 | 191 | **0** |
| 0.93 | 0.05 | 10 | 508 | 1419 | 487 | 1322 | 97 | 191 | **0** |

## Held-out check (requests + flask)

| score | threshold | margin | min tokens | silent right | **silent WRONG** | twin cases (silently wrong) | asked | missed | wrong silent if threshold −0.02 / −0.05 / −0.10 (train+test) |
|---|---|---|---|---|---|---|---|---|---|
| Edit | 0.99 | 0.00 | 10 | 423 | **0** | 388 (0) | 4134 | 23 | 186 / 399 / 604 |
| Jaccard | 0.91 | 0.05 | 10 | 504 | **0** | 388 (0) | 2586 | 1459 | 56 / 148 / 241 |

## Breakdown, Edit score (threshold 0.99, margin 0.00, min tokens 10), train + held-out

| edit level | placement | cases | silent right | asked | missed | wrong |
|---|---|---|---|---|---|---|
| 0% | SameScope | 480 | 316 | 162 | 2 | 0 |
| 0% | ModuleLevel | 365 | 210 | 154 | 1 | 0 |
| 0% | MovedFile | 480 | 277 | 201 | 2 | 0 |
| 5% | SameScope | 476 | 20 | 455 | 1 | 0 |
| 5% | ModuleLevel | 361 | 13 | 348 | 0 | 0 |
| 5% | MovedFile | 476 | 20 | 455 | 1 | 0 |
| 10% | SameScope | 474 | 3 | 470 | 1 | 0 |
| 10% | ModuleLevel | 361 | 2 | 359 | 0 | 0 |
| 10% | MovedFile | 474 | 3 | 470 | 1 | 0 |
| 20% | SameScope | 471 | 1 | 469 | 1 | 0 |
| 20% | ModuleLevel | 358 | 1 | 356 | 1 | 0 |
| 20% | MovedFile | 471 | 1 | 469 | 1 | 0 |
| 35% | SameScope | 467 | 0 | 465 | 2 | 0 |
| 35% | ModuleLevel | 356 | 0 | 355 | 1 | 0 |
| 35% | MovedFile | 467 | 0 | 465 | 2 | 0 |
| 50% | SameScope | 467 | 0 | 466 | 1 | 0 |
| 50% | ModuleLevel | 356 | 0 | 355 | 1 | 0 |
| 50% | MovedFile | 467 | 0 | 466 | 1 | 0 |
| 70% | SameScope | 467 | 0 | 466 | 1 | 0 |
| 70% | ModuleLevel | 356 | 0 | 355 | 1 | 0 |
| 70% | MovedFile | 467 | 0 | 466 | 1 | 0 |
| (deleted) | Deleted | 478 | silent 0 | asked 261 | – | – |

## Breakdown, Jaccard score (threshold 0.91, margin 0.05, min tokens 10), train + held-out

| edit level | placement | cases | silent right | asked | missed | wrong |
|---|---|---|---|---|---|---|
| 0% | SameScope | 480 | 394 | 85 | 1 | 0 |
| 0% | ModuleLevel | 365 | 288 | 76 | 1 | 0 |
| 0% | MovedFile | 480 | 277 | 202 | 1 | 0 |
| 5% | SameScope | 476 | 24 | 447 | 5 | 0 |
| 5% | ModuleLevel | 361 | 16 | 340 | 5 | 0 |
| 5% | MovedFile | 476 | 20 | 451 | 5 | 0 |
| 10% | SameScope | 474 | 3 | 466 | 5 | 0 |
| 10% | ModuleLevel | 361 | 2 | 354 | 5 | 0 |
| 10% | MovedFile | 474 | 3 | 466 | 5 | 0 |
| 20% | SameScope | 471 | 1 | 338 | 132 | 0 |
| 20% | ModuleLevel | 358 | 1 | 261 | 96 | 0 |
| 20% | MovedFile | 471 | 1 | 338 | 132 | 0 |
| 35% | SameScope | 467 | 0 | 174 | 293 | 0 |
| 35% | ModuleLevel | 356 | 0 | 148 | 208 | 0 |
| 35% | MovedFile | 467 | 0 | 174 | 293 | 0 |
| 50% | SameScope | 467 | 0 | 173 | 294 | 0 |
| 50% | ModuleLevel | 356 | 0 | 147 | 209 | 0 |
| 50% | MovedFile | 467 | 0 | 173 | 294 | 0 |
| 70% | SameScope | 467 | 0 | 173 | 294 | 0 |
| 70% | ModuleLevel | 356 | 0 | 147 | 209 | 0 |
| 70% | MovedFile | 467 | 0 | 173 | 294 | 0 |
| (deleted) | Deleted | 478 | silent 0 | asked 173 | – | – |

## Features: the true successor vs the best WRONG candidate (same-scope placement, mean over cases)

| edit level | true: jaccard | true: edit | true: name | true: params | best wrong: jaccard | best wrong: edit | best wrong: name |
|---|---|---|---|---|---|---|---|
| 0% | 0.95 | 0.96 | 0.64 | 1.00 | 0.27 | 0.46 | 0.14 |
| 5% | 0.71 | 0.91 | 0.64 | 0.84 | 0.28 | 0.46 | 0.14 |
| 10% | 0.54 | 0.86 | 0.64 | 0.72 | 0.28 | 0.46 | 0.14 |
| 20% | 0.30 | 0.77 | 0.64 | 0.43 | 0.27 | 0.46 | 0.14 |
| 35% | 0.10 | 0.62 | 0.64 | 0.12 | 0.27 | 0.46 | 0.14 |
| 50% | 0.06 | 0.58 | 0.64 | 0.04 | 0.28 | 0.46 | 0.14 |
| 70% | 0.06 | 0.58 | 0.64 | 0.04 | 0.27 | 0.46 | 0.14 |

# The provisional tier (Edit score)

Silent rule used: threshold 0.99, margin 0.00, min tokens 10, twin rule on. Cases that are still a soft question under it (not forced by a twin or a short body): 4086 train, 3729 held-out.

### Top-1 accuracy of the best candidate, per Edit-score band (any lead)

| Edit-score band | train: cases | train: top-1 right | train: accuracy | held-out: cases | held-out: top-1 right | held-out: accuracy |
|---|---|---|---|---|---|---|
| 0.30 – 0.35 | 23 | 0 | 0% | 13 | 0 | 0% |
| 0.35 – 0.40 | 17 | 6 | 35% | 11 | 0 | 0% |
| 0.40 – 0.45 | 20 | 6 | 30% | 4 | 0 | 0% |
| 0.45 – 0.50 | 43 | 33 | 77% | 68 | 55 | 81% |
| 0.50 – 0.55 | 105 | 92 | 88% | 171 | 167 | 98% |
| 0.55 – 0.60 | 397 | 373 | 94% | 458 | 442 | 97% |
| 0.60 – 0.65 | 476 | 456 | 96% | 514 | 493 | 96% |
| 0.65 – 0.70 | 316 | 252 | 80% | 229 | 198 | 86% |
| 0.70 – 0.75 | 85 | 52 | 61% | 121 | 90 | 74% |
| 0.75 – 0.80 | 381 | 325 | 85% | 340 | 301 | 89% |
| 0.80 – 0.85 | 238 | 125 | 53% | 300 | 198 | 66% |
| 0.85 – 0.90 | 542 | 393 | 73% | 490 | 378 | 77% |
| 0.90 – 0.95 | 827 | 550 | 67% | 648 | 535 | 83% |
| 0.95 – 1.00 | 616 | 292 | 47% | 362 | 292 | 81% |

### The same, only where the lead over the second best is at least 0.10

| Edit-score band | train: cases | train: top-1 right | train: accuracy | held-out: cases | held-out: top-1 right | held-out: accuracy |
|---|---|---|---|---|---|---|
| 0.30 – 0.35 | 7 | 0 | 0% | 2 | 0 | 0% |
| 0.35 – 0.40 | 8 | 6 | 75% | 3 | 0 | 0% |
| 0.40 – 0.45 | 7 | 6 | 86% | 1 | 0 | 0% |
| 0.45 – 0.50 | 40 | 33 | 82% | 67 | 55 | 82% |
| 0.50 – 0.55 | 94 | 89 | 95% | 154 | 152 | 99% |
| 0.55 – 0.60 | 327 | 321 | 98% | 424 | 421 | 99% |
| 0.60 – 0.65 | 425 | 424 | 100% | 484 | 472 | 98% |
| 0.65 – 0.70 | 238 | 222 | 93% | 202 | 186 | 92% |
| 0.70 – 0.75 | 60 | 49 | 82% | 97 | 87 | 90% |
| 0.75 – 0.80 | 335 | 304 | 91% | 301 | 291 | 97% |
| 0.80 – 0.85 | 160 | 113 | 71% | 205 | 162 | 79% |
| 0.85 – 0.90 | 452 | 346 | 77% | 371 | 349 | 94% |
| 0.90 – 0.95 | 699 | 489 | 70% | 527 | 472 | 90% |
| 0.95 – 1.00 | 379 | 257 | 68% | 277 | 277 | 100% |

### What a floor can reach at different accuracy targets

| target top-1 accuracy | floor | lead | train: links | train: accuracy | held-out: links | held-out: accuracy |
|---|---|---|---|---|---|---|
| 99% | – | – | none reaches it | | | |
| 95% | – | – | none reaches it | | | |
| 90% | 0.50 | 0.30 | 2183 | 91.0% | 2397 | 98.4% |
| 85% | 0.45 | 0.20 | 2681 | 85.6% | 2757 | 96.8% |
| 80% | 0.30 | 0.10 | 3231 | 82.3% | 3115 | 93.9% |

### Wrong top-1s among the tempting cases (Edit score >= 0.80, lead >= 0.10), by edit level

| set | edit level | cases | top-1 right | wrong: the function was DELETED | wrong: the real successor was outranked |
|---|---|---|---|---|---|
| train | deleted | 41 | 0 | 41 | 0 |
| train | 0-5% | 654 | 654 | 0 | 0 |
| train | 10-20% | 666 | 580 | 0 | 86 |
| train | 35%+ | 367 | 9 | 0 | 358 |
| held-out | deleted | 11 | 0 | 11 | 0 |
| held-out | 0-5% | 658 | 658 | 0 | 0 |
| held-out | 10-20% | 668 | 645 | 0 | 23 |
| held-out | 35%+ | 86 | 0 | 0 | 86 |

### No (floor, margin) reaches 95% top-1 accuracy with at least 30 cases on the training set. The reachable targets and what they would move:

### What moves from ask to provisional

| target accuracy | floor / lead | set | cases | asked before | asked after | **moved to provisional** | provisional right | provisional wrong | (of those: for a deleted function) | silent right (unchanged) |
|---|---|---|---|---|---|---|---|---|---|---|
| 90% | 0.50 / 0.30 | train | 4884 | 4354 | 2171 | **2183** | 1987 | 196 | 42 | 444 |
| 90% | 0.50 / 0.30 | held-out | 4711 | 4134 | 1737 | **2397** | 2359 | 38 | 14 | 423 |
| 85% | 0.45 / 0.20 | train | 4884 | 4354 | 1673 | **2681** | 2296 | 385 | 54 | 444 |
| 85% | 0.45 / 0.20 | held-out | 4711 | 4134 | 1377 | **2757** | 2670 | 87 | 27 | 423 |
| 80% | 0.30 / 0.10 | train | 4884 | 4354 | 1123 | **3231** | 2659 | 572 | 80 | 444 |
| 80% | 0.30 / 0.10 | held-out | 4711 | 4134 | 1019 | **3115** | 2924 | 191 | 44 | 423 |
