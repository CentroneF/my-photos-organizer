# Frame Brief: Library video-card previews

> Framing step before planning. This separates the visible symptom from its
> potential causes.

## Reported Observation

Video cards in Library Search appear blank until the user presses Play. The
video then appears.

## Initial Framing (preserved)

- **User's stated cause or approach**: No cause was specified.
- **User's proposed direction**: Make video cards show their first frame.
- **Pre-dispatch narrowing**: Playback displays the video, so this is limited
  to the initial preview state; the leading concern is blank-before-interaction.

## Dimension Map

1. **Video-card markup and load policy** — the browser may receive metadata
   without decoding or painting a frame.
2. **Asset URL and permission scope** — the browser may be unable to fetch the
   managed file.
3. **Card layout** — a decoded frame could be visually hidden.

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| Markup/load policy prevents an initial frame | The card uses `preload="metadata"` without a poster, first-frame decode path, or load handler at `src/app.rs:1029`. | STRONG |
| Asset URL or scope blocks previews | `safe_preview_url` validates, scopes, and returns an asset URL only for readable managed media; a failure renders “Preview unavailable.” Playback succeeds. `src-tauri/src/search.rs:264-304`. | NONE |
| CSS hides the frame | The card has a 4:3 preview box and its video fills that box, so it is not collapsed or hidden. `assets/styles.css:88`. | NONE |

## Narrowing Signals

- The user confirmed that pressing Play displays the video.
- Therefore the managed file, asset URL, scope, and codec playback path are
  available; the missing behavior is pre-play frame presentation.

## Cross-System Convention

The catalogue currently serves original video files directly and deliberately
does not generate thumbnails or posters. A reliable initial visual must be
handled at the card/browser presentation boundary or by adding an explicit
derived preview strategy in a later scoped change.

## Reframed Problem Statement

> **The actual problem to plan around is**: Library Search asks the browser for
> video metadata only and has no guaranteed pre-play visual representation.

This is not an asset-access or video-playback failure. Any follow-up should
ensure each available video card has a dependable pre-play representation,
while retaining the existing unavailable-preview fallback.

## Confidence

- **HIGH** — playback is confirmed, delivery/scoping is validated in code, and
  the only library-card-specific load policy is metadata-only.

## What Changes for /10x-plan

Plan a vertical, manually verifiable video-preview behavior change. Preserve
the imported-only boundary and do not grant additional filesystem access.

## References

- Source files: `src/app.rs:1029`, `assets/styles.css:88`,
  `src-tauri/src/search.rs:264-304`
- Investigation tasks: `video_markup`, `video_delivery`
