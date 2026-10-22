# 0003 — Readable accent text in the light theme

Date: 2026-10-22. Status: accepted for M0.

## Context

The light accent `#0E9C8A` has insufficient contrast for small text on white. The plan requires
WCAG AA body-text contrast while keeping the shared brand colors.

## Decision

Keep the specified aqua colors for fills, borders and decorative marks. In the light desktop
and website themes, use `#087668` for small accent text. Use `#0B0F14` for text on aqua buttons
in both themes, including Android.

## Consequences

Small links, labels and buttons meet the text-contrast requirement without changing the mark or
brand fills. The darker text color is an accessibility companion to the light aqua token.
