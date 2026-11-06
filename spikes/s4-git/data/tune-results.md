# S4 tuning results

train = LightRAG + httpx (4884 cases), held-out = requests + flask (4711 cases). ask_floor 0.30.

## Edit score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): 24 of 1440

| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |
|---|---|---|---|---|---|---|---|---|---|
| 0.99 | 0.00 | 10 | 445 | 3052 | 797 | 0 | 149 | 191 | **95** |
| 0.99 | 0.10 | 10 | 435 | 3062 | 815 | 0 | 149 | 191 | **77** |
| 0.99 | 0.05 | 10 | 435 | 3062 | 803 | 0 | 149 | 191 | **89** |
| 0.99 | 0.15 | 10 | 431 | 3066 | 824 | 0 | 149 | 191 | **68** |
| 0.99 | 0.30 | 10 | 427 | 3070 | 842 | 0 | 149 | 191 | **50** |

## Jaccard score: configs with ZERO wrong silent matches on the training set (twin cases judged apart): 212 of 1440

| threshold | margin | min tokens | silent right | asked (top right) | asked (in list) | missed | deleted asked | twin cases | **twin cases silently WRONG** |
|---|---|---|---|---|---|---|---|---|---|
| 0.91 | 0.05 | 10 | 529 | 1398 | 436 | 1322 | 92 | 191 | **92** |
| 0.92 | 0.05 | 10 | 523 | 1404 | 436 | 1322 | 92 | 191 | **92** |
| 0.92 | 0.00 | 10 | 523 | 1404 | 433 | 1322 | 92 | 191 | **95** |
| 0.91 | 0.10 | 10 | 513 | 1414 | 439 | 1322 | 92 | 191 | **89** |
| 0.91 | 0.15 | 10 | 511 | 1416 | 439 | 1322 | 92 | 191 | **89** |

## Held-out check (requests + flask)

| score | threshold | margin | min tokens | silent right | **silent WRONG** | twin cases (silently wrong) | asked | missed | wrong silent if threshold −0.02 / −0.05 / −0.10 (train+test) |
|---|---|---|---|---|---|---|---|---|---|
| Edit | 0.99 | 0.00 | 10 | 423 | **0** | 388 (51) | 4083 | 23 | 186 / 399 / 604 |
| Jaccard | 0.91 | 0.05 | 10 | 506 | **0** | 388 (51) | 2533 | 1459 | 56 / 148 / 241 |

## Breakdown, Edit score (threshold 0.99, margin 0.00, min tokens 10), train + held-out

| edit level | placement | cases | silent right | asked | missed | wrong |
|---|---|---|---|---|---|---|
| 0% | SameScope | 480 | 317 | 159 | 2 | 0 |
| 0% | ModuleLevel | 365 | 210 | 152 | 1 | 0 |
| 0% | MovedFile | 480 | 277 | 199 | 2 | 0 |
| 5% | SameScope | 476 | 20 | 451 | 1 | 0 |
| 5% | ModuleLevel | 361 | 13 | 344 | 0 | 0 |
| 5% | MovedFile | 476 | 20 | 451 | 1 | 0 |
| 10% | SameScope | 474 | 3 | 462 | 1 | 0 |
| 10% | ModuleLevel | 361 | 2 | 351 | 0 | 0 |
| 10% | MovedFile | 474 | 3 | 462 | 1 | 0 |
| 20% | SameScope | 471 | 1 | 461 | 1 | 0 |
| 20% | ModuleLevel | 358 | 1 | 348 | 1 | 0 |
| 20% | MovedFile | 471 | 1 | 461 | 1 | 0 |
| 35% | SameScope | 467 | 0 | 457 | 2 | 0 |
| 35% | ModuleLevel | 356 | 0 | 347 | 1 | 0 |
| 35% | MovedFile | 467 | 0 | 457 | 2 | 0 |
| 50% | SameScope | 467 | 0 | 458 | 1 | 0 |
| 50% | ModuleLevel | 356 | 0 | 347 | 1 | 0 |
| 50% | MovedFile | 467 | 0 | 458 | 1 | 0 |
| 70% | SameScope | 467 | 0 | 458 | 1 | 0 |
| 70% | ModuleLevel | 356 | 0 | 347 | 1 | 0 |
| 70% | MovedFile | 467 | 0 | 458 | 1 | 0 |
| (deleted) | Deleted | 478 | silent 0 | asked 253 | – | – |

## Breakdown, Jaccard score (threshold 0.91, margin 0.05, min tokens 10), train + held-out

| edit level | placement | cases | silent right | asked | missed | wrong |
|---|---|---|---|---|---|---|
| 0% | SameScope | 480 | 397 | 81 | 1 | 0 |
| 0% | ModuleLevel | 365 | 290 | 73 | 1 | 0 |
| 0% | MovedFile | 480 | 277 | 201 | 1 | 0 |
| 5% | SameScope | 476 | 24 | 443 | 5 | 0 |
| 5% | ModuleLevel | 361 | 16 | 336 | 5 | 0 |
| 5% | MovedFile | 476 | 20 | 447 | 5 | 0 |
| 10% | SameScope | 474 | 3 | 458 | 5 | 0 |
| 10% | ModuleLevel | 361 | 2 | 346 | 5 | 0 |
| 10% | MovedFile | 474 | 3 | 458 | 5 | 0 |
| 20% | SameScope | 471 | 1 | 330 | 132 | 0 |
| 20% | ModuleLevel | 358 | 1 | 253 | 96 | 0 |
| 20% | MovedFile | 471 | 1 | 330 | 132 | 0 |
| 35% | SameScope | 467 | 0 | 166 | 293 | 0 |
| 35% | ModuleLevel | 356 | 0 | 140 | 208 | 0 |
| 35% | MovedFile | 467 | 0 | 166 | 293 | 0 |
| 50% | SameScope | 467 | 0 | 165 | 294 | 0 |
| 50% | ModuleLevel | 356 | 0 | 139 | 209 | 0 |
| 50% | MovedFile | 467 | 0 | 165 | 294 | 0 |
| 70% | SameScope | 467 | 0 | 165 | 294 | 0 |
| 70% | ModuleLevel | 356 | 0 | 139 | 209 | 0 |
| 70% | MovedFile | 467 | 0 | 165 | 294 | 0 |
| (deleted) | Deleted | 478 | silent 0 | asked 165 | – | – |

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
