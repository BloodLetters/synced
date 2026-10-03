# AGENTS.md — Development Guidelines for `synced`

## 1. Project Overview
`synced` is a high-performance, segmented download manager built with Rust, designed for concurrent chunk downloading, pausing, and resuming.

---

## 2. Workspace Architecture
The repository must be organized as a Cargo workspace with distinct responsibilities:

```
synced/
├── Cargo.toml                # Workspace manifest
├── crates/
│   ├── synced-core/          # Core engine (probing, chunking, I/O, event streaming)
│   ├── synced-cli/           # CLI frontend (arguments, terminal progress display)
│   └── synced-gui/           # GUI frontend (desktop interface)
```

### Module Boundaries
* **`synced-core`**: Contains all business logic (HTTP probe, range chunk calculation, direct byte writing, download state machine). It must remain completely agnostic of CLI or GUI dependencies.
* **`synced-cli`**: Thin consumer crate using `clap` and `indicatif`. Only handles user input and renders terminal UI based on core events.
* **`synced-gui`**: Thin consumer crate. Interacts with `synced-core` via async channels and state handles.

---

## 3. Strict Coding Conventions

### A. Comments Policy
* Do not write redundant or trivial comments.
* Never place explanatory comments inside function bodies.
* If a comment is strictly necessary, place a concise doc comment (`///`) directly above the function signature or struct definition.

### B. Function Design & Reusability
* Every function must serve a reusable, distinct purpose. No one-off throwaway wrappers.
* Prefer pure, testable functions for calculations (e.g., segment boundary math, byte formatting).
* Keep signatures decoupled: accept standard types or traits (`AsyncRead`, `AsyncWrite`, `Path`) rather than tightly coupled monolithic states.

### C. File & Module Granularity (No God Files)
* Keep files under 250 lines. Every file must represent a single, focused concern:
  * `probe.rs`: Server capabilities and header inspection.
  * `segment.rs`: Chunk splitting and byte-range calculations.
  * `storage.rs`: Pre-allocation and offset-based disk writing.
  * `downloader.rs`: Task orchestration and worker lifecycle.
  * `events.rs`: Progress reporting and state events.
* Do not accumulate unrelated helper functions into a generic `utils.rs`. Group utilities by domain.

### D. Test Organization
* Do not place inline test functions inside source files (`src/`).
* All unit and integration test functions must reside strictly inside dedicated `tests/` directories within their respective crates:
  * `crates/synced-core/tests/`
  * `crates/synced-cli/tests/`

---

## 4. Concurrency & I/O Standards
* Use `tokio` for async orchestration.
* Write downloaded chunks directly to their exact byte offsets (`seek` or positioned write) on pre-allocated files. Avoid merging temporary files at the end.
* Communicate progress from workers to frontends via decoupled async channels (`tokio::sync::mpsc`).
