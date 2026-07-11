# Observatory Animated Fallback - Reflection

- Goal: provide a visually faithful fallback without moving Three.js or a
  JavaScript render loop onto the main thread.
- Outcome: achieved. The existing fallback owner now renders a complete inline
  SVG and CSS-only motion while the successful Worker scene remains unchanged.
- Deeper cause addressed: the old fallback represented only two diagonal lines,
  so capability failure discarded the observatory identity. The repair replaced
  that presentation at its canonical template/CSS owner.
- Review learning: failure must be terminal. A queued Worker `ready` message
  could otherwise overwrite fallback state after termination; the bridge now
  ignores every later Worker message and an executable VM test covers it.
- Complexity learning: 30 node animations were unnecessary and outside the
  minimum plan. Fixed node opacity preserves the visual density with less paint
  pressure and only three fallback keyframe families.
- Performance learning: raw Long Tasks API output was insufficient for
  attribution. A scene-disabled control plus CDP timeline showed the >200 ms
  task is the page's existing initial Layout work; measured scene increments
  remained below the approved threshold.
- Compatibility: fallback intentionally has no hover or click. Worker
  interaction, pause/resume, datasets, public routes, registry ownership,
  locales, MCP, Scalar, and auth behavior remain unchanged.
- Retirement: the two-line-only fallback is retired. No Canvas 2D renderer,
  main-thread Three.js path, compatibility alias, or second data owner remains.
- Residual risk: the page-level initial Layout long task remains a separate
  performance follow-up. The one-time Worker `readPixels` cost also remains as
  recorded by the parent Worker migration evidence.
- Decision: final independent review and workspace checks approved the
  completion candidate; the durable evidence commit is the remaining handoff step.

Method Pack output does not grant completion authority.
