# Similarity threshold discussion

## Observation

Three classroom photos were visually related but were not surfaced as similar by the app.

## Current behavior

- The app creates a 64-bit difference hash (dHash) from a 9×8 grayscale image.
- Two images match when their Hamming distance is at most `10` bits.
- Comparison only considers already-imported items that already have a visual hash; pending photos are not compared with one another, and older imports are not backfilled automatically.
- The photos may exceed the threshold because minor reframing, subject movement, background changes, and exposure differences substantially alter the tiny grayscale representation.

## Threshold as a runtime setting

The threshold can be exposed as a runtime setting without re-importing or re-hashing images, provided the dHash algorithm stays unchanged.

Important implementation detail: the current data model stores the threshold alongside each hash and the query requires the stored threshold to equal the current one. Raising the setting would therefore exclude older hashes unless the query is changed to use the dHash algorithm/version as its compatibility condition and apply the selected threshold at comparison time.

## Performance implications

- Changing the threshold itself is negligible: each candidate comparison is a 64-bit XOR plus bit count.
- A higher threshold does not make individual comparisons slower, but it produces more matches to render and review.
- Current matching scans eligible imported image hashes linearly. This is practical for small and moderate libraries; very large libraries may eventually need a candidate-prefilter/index strategy.
- Higher thresholds improve same-moment recall but increase false positives. Useful initial presets could be Strict (`8`), Balanced (`10`), and Broad (`14` or `16`).

## Suggested next step

Create a separate planned change to make the threshold configurable, remove threshold equality as a hash-compatibility gate, and add a calibrated photo fixture set that measures both recall and false-positive behavior.
