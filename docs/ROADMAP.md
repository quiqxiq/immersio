# Roadmap

Planned work that is not implemented yet. Completed work lives in git history; the note
below stays as the design contract for the shipped chat panel.

## AI chat panel

**Status:** Implemented. Kept here as the design contract for future changes.

**Goal:** A global chat drawer that answers questions about the article, video, or song
the user explicitly attaches, and works as a general English tutor with no context.

### Entry point and layout

- The chat button is in the titlebar's right cluster, before the account menu. It opens
  a right drawer; there is no new page and no left-sidebar item.
- The drawer is about 26rem wide, overlays the content area, and uses a dismissable scrim
  on narrow screens.
- Chat owns the right slot: opening it closes the queue, lyrics, and comments panels;
  opening any of those (including the now-playing tabs) closes chat. State is retained
  while theater mode hides the drawer.

### Context attachment

- Opening chat never attaches the current page or player item. General chat is the
  default.
- The drawer offers the currently available article, video, or song through an explicit
  "Attach" action. Only one attachment can be active at a time.
- The attached context is always visible as a chip with its label, source, and (for
  video/music) the position captured at attach or send time.
- Article priority is article > open video > playing music > none. The article
  registration is matched against both the route and its `url` query parameter.
- Video and music snapshots refresh their position when a message is sent. If the
  underlying item changes, the attachment auto-detaches instead of silently switching;
  the drawer explains what changed and offers the new content.
- Opening an old chat does not restore an attachment.
- Quick actions (`Summarize this article`, `Summarize up to here`, `Explain these
  lyrics`) attach the content first; their prompts stay English regardless of UI locale.

### Backend and streaming

- `ai_chat_stream` is stateless: it receives the recent turns and at most one current
  context, then reuses the normal OpenRouter → NVIDIA NIM → Cloudflare fallback chain,
  cooldowns, SSE parser, and dynamic free-model discovery.
- Context text is capped at 18k characters, history at 12k characters and 40 turns,
  and output at 1,200 tokens. Oversized source text is marked as truncated.
- Attached text is wrapped in an `<attached_context>` block and treated as untrusted
  source material, not instructions. Delimiters and attribute values are escaped.
- Chat does not use or populate `ai_cache`.

### English-only answers

- A fixed server-side prompt requires every answer to be English. It cannot be replaced
  by the UI or overridden by a user message.
- After a stream completes, `whatlang` checks the prose (ignoring code, block quotes,
  inline code, URLs, and short answers). A non-English answer gets one corrective retry,
  then the normal provider fallback. `Done` is never sent with a final non-English answer.
- Streaming can briefly show a non-English draft before the reset/retry; buffered strict
  mode is intentionally out of scope for first-token latency.
- The word popup keeps its existing language behavior. Only the chat panel is
  English-only.

### History and privacy

- Recent chats are global, not per article/video/song, and persist in SQLite across app
  restarts. Users can open, delete one, or clear all.
- Stored per message: text, role, model, timestamp, and the context kind/id/label/detail.
  Article text, transcripts, and lyric bodies are never stored in chat history.
- History sends only message text to the backend; source material is sent only for the
  current turn's attachment.
- Saved API tokens remain backend-only and are never returned to or displayed by the
  frontend.

### Verification

- Rust tests cover context wrapping/truncation, history sanitization, the English guard,
  provider payloads, and SQLite chat CRUD.
- Frontend checks cover type safety and production build. Manual verification should cover
  streaming, attachment/detach, auto-detach, history, panel exclusivity, and the
  English-only prompts, plus a regression pass on the word popup.
