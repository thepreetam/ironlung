# IronLung Symbol Subset

IronLung targets a **curated subset** of libc, not full glibc compatibility. The set is defined by [crates/ironlung-abi-check/symbols.baseline](../crates/ironlung-abi-check/symbols.baseline) and documented here.

## Intended subset

- **Current baseline:** All symbols listed in `symbols.baseline` are implemented and exported. See [ARCHITECTURE.md](ARCHITECTURE.md) for delegation rules (kernel-delegating vs validate-then-delegate vs contained).
- **Next tier:** High-value symbols added over time (e.g. string helpers, env) to unblock more real programs under LD_PRELOAD. No commitment to implementing every glibc symbol.
- **Out of scope:** Full locale, wide char beyond minimal delegation, full stdio buffering reimplementation, and anything not listed in the baseline or in the "next tier" list below.

## Next tier (candidates / added)

| Symbol    | Status   | Notes                          |
|----------|----------|---------------------------------|
| memset   | added    | Validate size, delegate to libc |
| strcmp   | added    | Bounded length check, delegate  |
| strncpy  | added    | Validate n, delegate            |
| snprintf | added    | Validate format/size, delegate  |
| getenv   | added    | Validate name length, delegate  |
| wcslen   | added    | Bounded scan, delegate         |
| wcscpy   | added    | Delegate                       |
| wcsncpy  | added    | Validate n, delegate            |
| wcscmp   | added    | Delegate                       |

## Process for new symbols

1. **Propose:** Open an issue or add a row to the "Next tier" table above with symbol name and rationale.
2. **Implement:** Add validate-then-delegate (or kernel path if it fits) in the appropriate module; ensure bounds/null checks.
3. **Baseline:** Add the symbol to [crates/ironlung-abi-check/symbols.baseline](../crates/ironlung-abi-check/symbols.baseline).
4. **Doc:** Update this file and [README.md](../README.md) Implemented section; run ABI check and CI.
