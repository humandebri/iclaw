# iclaw ICP Integration Tests

This directory contains canister integration tests for `iclaw`.

## What it covers

- `health()` query behavior with and without provider config
- `memory_*` round-trip behavior on PocketIC
- `allowed_principals_get/set()` update and upgrade persistence
- `run_create()` success/failure behavior before mainnet verification

## Run

```bash
npm install
npm test
```

## Notes

- The tests expect `target/ic/iclaw.wasm` to exist.
- Build the Wasm first with `icp build iclaw`.
- The `run_create()` configured-provider case uses PocketIC HTTPS outcall mocks to verify that the canister enters the provider path and still returns `Ok(Run)` with `status = "failed"` and `error = "provider_error..."` when the upstream request fails.
- These tests validate canister API behavior, not real upstream HTTPS outcall success.

## ContextConfig payload compaction contract

- `max_prompt_chars` keeps the existing chars-based guard.
- `max_request_bytes_budget` adds a lower preflight budget before the transport-level `MAX_REQUEST_BYTES` limit.
- `llm_summary_request_bytes_threshold` only gates the overflow-only LLM summary path.
- If only `chars` overflow, the runtime trims body and context sections without calling the summary model.
- The normal path does not call the summary model; LLM summary is reserved for bytes-risk overflow.
- Compaction order is fixed: drop older body messages, trim session summary, trim memory context, optionally run one LLM summary attempt, then drop more body messages as the final fallback.
