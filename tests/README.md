# iclaw ICP Integration Tests

This directory contains canister integration tests for `iclaw`.

## What it covers

- `health()` query behavior with and without provider config
- `memory_*` round-trip behavior on PocketIC
- `chat()` error behavior before mainnet verification

## Run

```bash
npm install
npm test
```

## Notes

- The tests expect `target/ic/iclaw.wasm` to exist.
- Build the Wasm first with `icp build iclaw`.
- The `chat()` configured-provider case uses PocketIC HTTPS outcall mocks to verify that the canister enters the provider path and normalizes the failure as `provider_error`.
- These tests validate canister API behavior, not real upstream HTTPS outcall success.

## ContextConfig payload compaction contract

- `max_prompt_chars` keeps the existing chars-based guard.
- `max_request_bytes_budget` adds a lower preflight budget before the transport-level `MAX_REQUEST_BYTES` limit.
- `llm_summary_request_bytes_threshold` only gates the overflow-only LLM summary path; the normal path does not call the summary model.
- Compaction order is fixed: drop older body messages, trim session summary, trim memory context, optionally run one LLM summary attempt, then drop more body messages as the final fallback.
