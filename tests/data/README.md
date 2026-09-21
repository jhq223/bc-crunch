# Fixtures

`dictionary-reference.bcc` is the BC5 stream produced from `bc5.bc` by the Rust
encoder before the dictionary early-exit optimization. The exact-byte check
protects selector ordering and dictionary tie-breaking during refactoring.
The fixture is synthetic and contains no game assets.
