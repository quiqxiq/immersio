<div align="center">

# 🌐 Immersio

**Desktop Language Immersion & Comprehensible Input Media Suite**  
*Watch, listen, read, and acquire languages naturally with interactive subtitles, AI tutoring, and 1-click Anki mining.*

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri_2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white">
  <img alt="Svelte 5" src="https://img.shields.io/badge/Svelte_5-FF3E00?style=for-the-badge&logo=svelte&logoColor=white">
  <img alt="libmpv" src="https://img.shields.io/badge/libmpv-8A2BE2?style=for-the-badge">
  <img alt="License" src="https://img.shields.io/badge/License-GPL_v3-blue?style=for-the-badge">
  <br>
  <img alt="Linux" src="https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black">
  <img alt="Windows" src="https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white">
  <img alt="macOS" src="https://img.shields.io/badge/macOS-000000?style=for-the-badge&logo=apple&logoColor=white">
</p>

</div>

---

### 💡 Project Origin & Fork Attribution

> **Immersio** is developed as an advanced fork of [**Limusic**](https://github.com/SimoHypers/limusic) created by [@SimoHypers](https://github.com/SimoHypers).
> 
> While **Limusic** is an exceptional, fast, ad-free desktop client for YouTube Music, **Immersio** transforms and expands this powerful engine into an all-in-one **Language Immersion & Comprehensible Input platform**. 
> 
> Full appreciation and credit go to **SimoHypers** and the contributors of Limusic and Metrolist for the foundational libmpv integration, YouTube API reverse-engineering, and elegant audio streaming architecture!

---

## 🚀 Key Immersion Features

Immersio empowers language learners to achieve natural fluency through authentic content:

### 1. 🎬 Interactive Clickable Video Transcripts
- **Closed Captions & Timed Transcripts:** Video transcripts sync seamlessly with playback.
- **Word-Level Interactivity:** Click any word in a subtitle or transcript line to immediately inspect its definition, grammatical role, and CEFR difficulty level.
- **Auto-Pause on Lookup:** The video pauses automatically when you click a word, allowing you to absorb new vocabulary at your own pace.

### 2. 🤖 AI Immersion Tutor
- **Contextual Sentence Breakdowns:** Ask the AI tutor to explain what a specific word or phrase means *in the exact context of the video sentence*.
- **Idioms & Colloquialisms:** Unpacks slang, phrasal verbs, cultural nuances, and figurative language that traditional dictionaries miss.
- **Multi-Model Provider Support:** Compatible with **OpenAI (GPT-4o)**, **Anthropic (Claude 3.5 Sonnet)**, **Google Gemini**, **Groq (Llama 3)**, and local private models via **Ollama**.

### 3. 🗃️ 1-Click Anki SRS Mining
- **Instant Card Creation:** Click *"Kirim ke Anki"* on any word popup to generate an Anki flashcard instantly via AnkiConnect.
- **Rich Card Formatting:** Cards include cloze deletion (`[...]`), sentence context, timestamps, target word, definition, and CEFR level tags.

### 4. 📚 Built-in Multi-Engine Dictionaries
- **Offline & Instant:** Integrated with ECDIC and local lemmatizers for zero-latency lookups.
- **CEFR Difficulty Tags:** Every word displays its CEFR rating (`A1` to `C2`) and frequency rank so you know whether to prioritize learning it.
- **Online Fallback:** Automated lookup fallback to Wiktionary and contextual definitions.

### 5. 📺 Curated Educational & Tech Videos Hub
- **Dedicated Immersion Hub:** Browse feeds curated for language learning (Technology, AI, Science, TED Talks, Podcasts, and Discussions).
- **Accurate Metadata Parsing:** Real creator channel names, high-resolution thumbnails, and video durations.
- **Hardware-Accelerated Native Video:** Smooth H.264 playback accelerated via VA-API on AMD, Intel, and Nvidia GPUs without CPU spikes or frame drops.

### 6. 📰 Immersion Article & Reading Suite (3-Step Local Pipeline)
- **3-Step Local Extraction Pipeline:**
  1. **Smart Fetching:** `reqwest` with realistic browser headers to prevent anti-bot false alarms.
  2. **Editorial Content Extraction:** Mozilla Readability ported to native Rust — automatically strips ads, sidebars, cookie banners, and popups.
  3. **Markdown Transformation (`htmd`):** Converts clean HTML into standard, portable Markdown.
- **Curated Open-Access & Tech Sources (100% Paywall-Free):**
  - **Dev.to:** Official REST API integration with real-time software engineering and developer articles.
  - **Hacker News:** Official Algolia API integration with top trending tech stories, essays, and discussions.
  - **FreeCodeCamp News:** Comprehensive programming tutorials and deep dives.
  - **NPR News:** High-register standard American journalism on world news, science, and culture.
  - **The Guardian:** Open British international journalism across world affairs and science.
  - **BBC Learning English:** Specially tailored bite-sized episodes with vocabulary definitions built-in.
- **Interactive Immersion & Note Export:**
  - Click any word for Cambridge dictionary definitions, pronunciation TTS, and 1-click AnkiSRS mining.
  - Sentence-level hover highlighting matching the video transcript experience.
  - One-click **"Salin MD"** to export articles directly to Obsidian, Notion, or Logseq for language retention.
  - Automatic detection of paywall / bot-challenge stubs with a friendly fallback to the original browser.

---

## 🎵 Full Media Player Engine (Inherited from Limusic)

Underneath the immersion suite is the robust, high-performance Limusic audio core:

- **Ad-free streams**: Direct from YouTube's API with zero audio advertisements.
- **Gapless playback**: Powered by `libmpv` with loudness normalization and configurable crossfades.
- **Synced lyrics**: Word-by-word synced karaoke lyrics with integrated translations.
- **Your Library & Playlists**: Sign in with Google / YouTube Music accounts, manage playlists, view history, and like tracks.
- **Desktop Integrations**: Discord Rich Presence, MPRIS (Linux) & SMTC (Windows) media keys, and system tray minimization.
- **18+ UI Languages**: Fully localized interface across English, Indonesian, Spanish, Chinese, Japanese, and more.

---

## 🛠️ Development & Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (edition 2021)
- [Node.js](https://nodejs.org/) (v18+) and [pnpm](https://pnpm.io/)
- `libmpv` development libraries:
  - **Fedora / RHEL:** `sudo dnf install mpv-libs-devel webkit2gtk4.1-devel`
  - **Ubuntu / Debian:** `sudo apt install libmpv-dev libwebkit2gtk-4.1-dev`
  - **Arch Linux:** `sudo pacman -S mpv webkit2gtk-4.1`

### Running Locally

1. **Clone the repository:**
   ```bash
   git clone https://github.com/quiqxiq/immersio.git
   cd immersio
   ```

2. **Install frontend dependencies:**
   ```bash
   pnpm install
   ```

3. **Run the desktop app in development mode:**
   ```bash
   pnpm dev
   ```

---

## 📄 License

Immersio is open-source software licensed under the **GNU General Public License v3.0 (GPLv3)**, inheriting and honoring the open-source license of [Limusic](https://github.com/SimoHypers/limusic).
