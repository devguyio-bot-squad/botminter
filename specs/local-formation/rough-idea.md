# Local Formation as First-Class Concept

Implement ADR-008: Refactor the existing `formation/` module to introduce a `Formation` trait, restructure into a `local/linux` platform hierarchy, move `LocalCredentialStore` from `bridge/` into the formation, and make commands formation-agnostic via `Box<dyn Formation>`.

Source ADR: `.planning/adrs/0008-local-formation-as-first-class-concept.md`
