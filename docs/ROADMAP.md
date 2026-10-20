# Roadmap

Planned work that is not implemented yet. Completed work lives in git history.

## Context-aware AI chat panel

**Status:** Planned — proposed design, implementation not started.

**Goal:** A chat panel that can answer questions about the article, video, or music
currently open, and falls back to general conversation when nothing is open.

### UX

- A chat button in the titlebar's right cluster, alongside the account and integration
  icons. It opens a right-side panel; the left sidebar stays untouched.
- The panel is a right drawer sized for conversation (about 26rem). It should reserve
  space on wide screens and overlay with a dismissable scrim on narrow ones.
- A context card at the top shows what the assistant can see:
  - article title and source;
  - video title, channel, and playback position;
  - track title, artist, album, and playback position;
  - "General mode" when there is no active content.
- Users can detach the context and keep chatting generally.
- Starter prompts adapt to the active content (summarise, explain vocabulary,
  explain a lyric, practise conversation, and so on).

### Context sources

- **Article:** the `/articles/read` page sets the active context after extraction
  succeeds, using clean text/Markdown rather than raw HTML. It clears the context when
  the page is left.
- **Video:** active only while the video player view is open, using the current track
  metadata and the transcript around the active sentence.
- **Music:** derived from the current track, with lyrics fetched on demand through the
  existing lyrics backend.
- **General:** no context payload.

Proposed priority: article > open video > playing music > general.

### Backend and privacy

- Add a streaming `ai_chat_stream` command that reuses the existing provider fallback
  chain, cooldowns, and SSE parser.
- Build the context prompt on the backend. Truncate long article, transcript, and lyric
  text; keep only the recent conversation turns to protect first-token latency.
- Do not add an extra AI summarisation call before answering, and do not cache chat in
  the word-explanation cache.
- API tokens remain backend-only. The frontend never receives or sends saved tokens.
- Show that context will be sent to the AI provider when a message is submitted.

### Open decisions

- Conversation history: one thread per context in memory, or one global thread?
- Should chat history persist across app restarts?
- Can the chat panel coexist with the queue/lyrics/comments panels, or should it be
  mutually exclusive with them in the first version?

### Acceptance criteria

- Opening an article, video, or track is reflected in the context card.
- Asking about the active content produces an answer grounded in that content.
- With nothing open, the panel works as a general chat.
- Responses stream token by token through the existing AI provider fallback.
- No saved API token is returned to or displayed by the frontend.
