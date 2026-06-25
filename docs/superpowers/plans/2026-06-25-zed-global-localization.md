# Zed Global Localization Plan

## Goal

Turn the current Chinese-only fork into `zed-global`: a multilingual Zed fork with compiled locale resources, BCP 47 locale tags, English source strings recovered from upstream, Simplified Chinese extracted from the existing hardcoded fork, and additional locale files produced by independent reviewable translation commits.

## Constraints

- Use upstream English strings from `upstream/main` or reachable git history, not back-translation from Chinese.
- Preserve existing Simplified Chinese phrasing as `zh-Hans`.
- Use BCP 47 / IETF locale tags:
  - Base: `en-US`, `zh-Hans`
  - Additional: `zh-Hant-HK`, `zh-Hant-TW`, `ja-JP`, `ko-KR`, `es-ES`, `ru-RU`
- Stable semantic keys, not source-English-as-key.
- Locale fallback order: exact locale, then language/script parent when available, then `en-US`.
- Preserve placeholders, keyboard shortcuts, action names, file names, command identifiers, and Rust format syntax.
- Do not use Google Translate or translation APIs for any locale work.

## Implementation Steps

1. Sync `upstream/main`, resolve merge conflicts, rename GitHub repo to `Ce-daros/zed-global`, and update `origin`.
2. Add a dedicated localization crate with:
   - compiled JSON resources,
   - locale selection,
   - exact/parent/English fallback,
   - placeholder interpolation,
   - tests that enforce identical key sets and placeholder compatibility.
3. Extract current fork translations by comparing current files against upstream versions:
   - upstream literal becomes `en-US`,
   - current Chinese literal becomes `zh-Hans`,
   - generated keys use UI domain prefixes such as `agent.thread.go_to_file`.
4. Replace the extracted hardcoded UI strings in the already-localized Rust surfaces with localization lookups.
5. Verify with focused tests and targeted cargo checks before committing the base system.
6. Dispatch one `gpt-5.4-mini` medium subagent per additional locale. Each subagent translates only its locale JSON and must translate manually as an LLM, not through Google Translate or any translation API.
7. Dispatch one review subagent after the locale commits. The review agent checks large blocks, placeholder parity, cultural wording, and fills untranslated or missed entries.
8. Commit each locale separately, rewrite README for `zed-global`, push all commits, trigger the manual release workflow, and stop after the workflow is queued.

## Commit Boundaries

- Upstream sync merge commit.
- Base localization runtime plus `en-US` and `zh-Hans`.
- One commit per added locale.
- README/release workflow metadata commit if needed.

## Verification

- Localization crate tests for fallback and placeholder handling.
- Locale-resource validation for complete key parity.
- Focused `cargo check` on touched crates where feasible.
- Final git status review before push.
