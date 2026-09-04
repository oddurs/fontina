## What

<!-- One paragraph. Title must be in Conventional Commits form: type(scope): subject -->

## Why

## How it was tested

- [ ] `cargo test` passes locally
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] Snapshots reviewed (`cargo insta review`) if metadata output changed
- [ ] `schemas/face.json` regenerated if the model changed

## Checklist

- [ ] No new dependency without a reason above
- [ ] No system font directories touched; no network; no telemetry
- [ ] Docs / ADR updated if a decision changed
