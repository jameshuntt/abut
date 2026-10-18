# Changelog

## 0.2.2 (2026-10-18)

0.2.2 moves abut onto liaise 0.2.4. 0.2.1 still asked for liaise ^0.1.6, so anything that used abut next to liaise 0.2 built liaise twice, and abut's codes implemented the old `Liaise` trait, not the one the rest of the tree uses. nothing else changed. every code and message renders byte for byte the same as 0.2.1.

### Cargo.toml

- liaise was 0.1.6: [Cargo.toml L25-L25](https://github.com/jameshuntt/abut/blob/496d0c3d63434c61669ba67888d7daa1ce86e6e6/Cargo.toml#L25-L25). it's 0.2.4 now, still with `std`: [Cargo.toml L25-L25](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/Cargo.toml#L25-L25). the derive under it goes from liaise_derive 0.2.3 to 0.3.4.
- version 0.2.1 to 0.2.2: [Cargo.toml L3-L3](https://github.com/jameshuntt/abut/blob/496d0c3d63434c61669ba67888d7daa1ce86e6e6/Cargo.toml#L3-L3), [Cargo.toml L3-L3](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/Cargo.toml#L3-L3).

### the derive

- liaise_derive 0.2.3 wrote `Display` as `self.render()`, so the `Liaise` trait had to be in scope wherever `#[derive(LiaiseCodes)]` was, and ring.rs imported it for that alone: [src/ring.rs L16-L16](https://github.com/jameshuntt/abut/blob/496d0c3d63434c61669ba67888d7daa1ce86e6e6/src/ring.rs#L16-L16). 0.3.4 writes `::liaise::Liaise::render(self)`, so that import went unused and is just `LiaiseCodes` now: [src/ring.rs L16-L16](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/src/ring.rs#L16-L16). error.rs keeps `Liaise` in its import, `AbutError`'s `Display` calls `self.code.render()` itself: [src/error.rs L149-L155](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/src/error.rs#L149-L155).
- the derive also gives `AbutCode` and `RingError` the rest of liaise 0.2's `Liaise` trait (`code`, `slug`, `severity`, `stability`, `descriptor`, `liaise_report` and the others), all defaults since abut sets none of them. `prefix` is a method in 0.2, `code.prefix()`, where it was `AbutCode::prefix()` in 0.1. abut never calls it. no abut item was added or removed: the `cargo doc` item list is the same 19 items before and after.

### proof

- the codes and messages are pinned by the tests that were already here, unchanged and all passing: [tests/framed.rs L56-L56](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/framed.rs#L56-L56), [tests/framed.rs L91-L91](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/framed.rs#L91-L91), [tests/framed.rs L118-L118](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/framed.rs#L118-L118), [tests/framed.rs L151-L155](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/framed.rs#L151-L155), [tests/codecs.rs L57-L57](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/codecs.rs#L57-L57), [tests/codecs.rs L114-L114](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/codecs.rs#L114-L114), [tests/ring.rs L65-L68](https://github.com/jameshuntt/abut/blob/10b2031c53694be5a4bed6c02ba74cc366bec7ff/tests/ring.rs#L65-L68). `cargo test --all-features` is 24 passed and 2 doctests passed, 1 ignored, the same as 0.2.1.
- parity: one program, built once against the published abut 0.2.1 and mine 0.3.1 (liaise 0.1.6) and once against abut 0.2.2 and mine 0.3.2 (liaise 0.2.4), prints `PREFIX`, `ALL_CODES`, `code_id`, `message`, `render` and `Display` for every `AbutCode` and `RingError` variant, and `Display` and `source` for an `AbutError` of every kind. the two outputs are the same 94 lines, sha256 33f2d9eea9226fc5cd7617b0f171b0743d385fd0c01b16b47b128add47782ca8.
