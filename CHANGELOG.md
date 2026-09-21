# Changelog

## 0.1.1

- Avoid building and updating decoder lookup tables during compression.
- Count selector runs in one pass and select only the most frequent dictionary entries.

## 0.1.0

- Lossless compression and decompression of BC1, BC3, BC4 and BC5 blocks.
- Decoding into caller-owned storage.
- Range-coded streams with explicit lookahead padding.
- Checked decoded-buffer sizing and optimized selector dictionary lookup.
