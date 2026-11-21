# Local Optimization Report: DevFest Warsaw 2026

## Overview
This document contrasts the upstream baseline implementation (`vibezone-starter:local`) with our TDD-driven optimized API entry (`devfest-api-optimized:local`), detailing changes, exact verification logs, and benchmark outcomes.

## Code Changes & TDD
- **Test-Driven Development:** Before modifying production code, we authored a comprehensive test suite in `starter/tests/test_app.py` verifying SQLite pragmas (`journal_mode=WAL`, `synchronous=FULL`), the existence of the unique index on `likes(post_id, user)`, feed single-query aggregation efficiency, and like idempotency.
- **SQLite Pragmas & Indexes:**
  - Configured WAL journal mode and `synchronous = FULL` globally and per-connection to strictly preserve durability and crash resilience.
  - Added `CREATE UNIQUE INDEX IF NOT EXISTS idx_likes_post_user ON likes(post_id, user);` to eliminate full table scans on like lookups and guarantee zero double-counting.
- **Single-Query Feed Aggregation:**
  - Replaced the $N+1$ query loop in `GET /feed` (which executed 21 separate queries per feed request) with a single efficient JOIN + GROUP BY subquery:
    ```sql
    SELECT p.id, p.author, p.content, p.created_at, COUNT(l.post_id) AS likes
    FROM (SELECT id, author, content, created_at FROM posts ORDER BY id DESC LIMIT 20) p
    LEFT JOIN likes l ON p.id = l.post_id
    GROUP BY p.id
    ORDER BY p.id DESC
    ```

## Verification & Test Results
- **Unit & Integration Tests:** All tests passed successfully inside the container:
  ```
  Ran 3 tests in 0.131s
  OK
  ```
- **Official API Contract & Durability Checks:**
  - 145/145 correctness checks passed successfully.
  - Crash recovery & restart durability verified successfully across multiple phases.

## Benchmark Comparison (Smoke Profile)
- **Baseline (`vibezone-starter:local`):**
  - Report `results/20261121T130454-fa5cb553.json`: 10 VU smoke p99 = 33.39 ms, throughput = 4.8 RPS.
- **Optimized (`devfest-api-optimized:local`):**
  - Image size: 41,157,016 bytes (uncompressed), matching baseline footprint.
  - Report `results/20261121T131651-1b664019.json`: 10 VU smoke p99 = 16.75 ms, throughput = 5.2 RPS, 0% errors; correctness and durability checks passed.

These are two short local smoke runs, not a statistically established speedup or an official U score. The observed p99 is about 50% lower in this pair; run-to-run noise and the tiny sample prevent generalizing that improvement. Both candidates passed only the tested smoke ceiling of 10 VUs. Higher-load measurements and the official runner are still required.

## Artifacts & Reports
- Optimized Image Tag: `devfest-api-optimized:local`
- Representative Report Path: `/home/matt/.hermes/cache/scratch/devfest-api-entry/upstream/results/20261121T131651-1b664019.json`
