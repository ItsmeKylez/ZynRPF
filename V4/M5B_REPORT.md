# M5b REPORT: step 1, DEFLATE (2026-11-24)
Scope: the user said "just continue to m5b or m5c idc". M5b was chosen (M5c needs the user's real assembly.xml, which does not exist yet). Only the first building block of M5b was done: compression in `rpf-core`. Text editing, compressed-entry replace, nested RPF and the header badges are NOT started.

## Built
- `crates/rpf-core/src/deflate.rs` (registered in lib.rs): `inflate(data, max_out)` (stored, fixed and dynamic blocks, every read bounds-checked, output capped by the caller, no recursion) and `deflate_stored(data)` (valid raw DEFLATE using stored blocks, never smaller than the input). Error type `InflateError` with English messages. 9 unit tests (zlib-made fixed, dynamic and stored vectors, 1 MiB zero bomb with limit, every truncation, reserved block type, bad stored length, 20,000 garbage inputs).
- `scripts/deflate_port_check.py`: the Python port used to validate the algorithm.
- D-048 (PROPOSED): own std-only implementation instead of a crate.

## VERIFIED (in this container, with Python)
- The algorithm, as a line-by-line Python port: 4,800 round trips against zlib (stored, fixed, dynamic, Huffman-only, RLE; sizes 0 to 70,000; random, text, zeros), 20,000 garbage inputs and every truncation point gave only clean errors, a 1 MiB zero bomb stopped by the limit.
- The test vectors embedded in the Rust tests decode correctly with that port; the file's brackets balance.

## NOT verified
- The Rust file was NEVER COMPILED or run: this container has no Rust toolchain. Expect possible compile errors; if `cargo test` fails, send the output and I will fix it.
- Not wired into the parser, server or UI. Not tested on real game data. Whether real RPF entries use raw DEFLATE (no zlib header) is assumed, not checked (UNVERIFIED). Speed was not measured (bit-by-bit decoding, likely tens of MB/s).
- Not done: a ratio limit (callers pass `max_out`, ideally the entry's declared size), mapping `InflateError` to new append-only check codes and the error catalogue (D-031), real compression.

## Next steps inside M5b (each needs your `APPROVED` naming it, or "you choose")
1. You run `cargo test --release` (or `scripts/run-all-checks.sh`) and send failures.
2. Wire inflate into reading compressed entries (raw bytes and text read, with limits and new check codes).
3. Text read/save endpoint and editor panel (saving as a stored-block replacement first).
4. Nested RPF read, then header badges (F-02). Real deflate compression last.

---
# Step 2: read compressed files as plain bytes, 2026-11-24
Scope: the user said "you choose" again. Chosen: connect the decoder to the existing read route, nothing more.

## Built
- `GET /api/v1/versions/{v}/entries/{id}/bytes?basis=uncompressed` (crates/server/src/api.rs, `entry_bytes`). Before, only `basis=stored` was accepted. Now, for a compressed plain file it: refuses entries whose encryption field is not 0 (`entry_encrypted`); refuses declared or stored sizes above `MAX_BLOB_BYTES` (`too_large_to_decompress`); reads the stored bytes, inflates them with the output cap set to the declared size (`inflate_failed` on a bad stream), requires the result to match the declared size (`inflate_size_mismatch`), then serves the requested Range (max 1 MiB per range, as before). Files that are not compressed fall through to the normal stored path. Existing behavior of `basis=stored` is unchanged.

## NOT verified
- NEVER COMPILED or run (no Rust toolchain here), same as deflate.rs. No new test covers this route: the synthetic generator only makes uncompressed entries, so an e2e test needs a generator change first.
- Assumptions that may be wrong on real files (UNVERIFIED): entries use raw DEFLATE; the per-entry encryption field is 0 for plain data (if it holds another value in real archives, every compressed file would be refused with `entry_encrypted`, which is safe but unhelpful).
- Each request re-decompresses the whole entry (up to `MAX_BLOB_BYTES`, default 256 MiB, in memory); fine for a first step, to be improved (cache or smaller cap) before public use. The new error codes are not in the English catalogue or API_SPEC yet.

## Next (each needs `APPROVED` naming it, or "you choose")
1. You run `sh scripts/run-all-checks.sh` and send failures (nothing from M5a part 4 on has been compiled or run).
2. Generator support for compressed entries plus an e2e test for this route.
3. Text editor panel (read through this route; save as a stored-block replacement).

---
# Step 3: compressed test archive and end-to-end test, 2026-11-24
Scope: the user said "you choose" again. Chosen: the test that step 2 lacked.

## Built
- `testgen::gen_compressed_to` (crates/rpf-core/src/testgen.rs) and `gen <out> compressed` (examples/gen.rs): a 4-entry archive with `fixed.txt` (real zlib fixed-Huffman data), `stored.txt` (stored blocks) and `bomb.bin` (data inflates to 100 bytes, table says 10).
- `scripts/e2e-m5b.mjs`: 10 checks (parse, listing, plain-size report, both files decompress to the expected text, Range on decompressed data, `basis=stored` and the default unchanged, bomb refused with `inflate_failed`, unknown basis refused). Added to `scripts/run-all-checks.sh`.

## VERIFIED
- With Python only: the payload logic (embedded zlib vector and stored wrapper decode to the expected text; the bomb is refused at the declared size), the e2e script parses (`node --check`), the shell script parses.

## NOT verified
- NOTHING from this step, step 2 or step 1 has been compiled or run (no Rust toolchain here). The generator's table layout was written from the existing `gen_to`, not run through `Index::parse`; the e2e test is the first real check of it.
- Real game files still untested; raw-DEFLATE and encryption-field assumptions from step 2 stand.

## Next
1. You run `sh scripts/run-all-checks.sh` and send any `out-*.log` failures. Everything since M5a part 4 is waiting on that.
2. Then: text editor panel (read through `basis=uncompressed`; save as stored-block replacement).
