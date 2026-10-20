//! Music video, played by mpv itself.
//!
//! The picture is one more track of the audio file mpv is already playing (`video-add`), so picture
//! and sound run off one clock: a seek, a pause or a tempo change moves both, and there is nothing
//! to keep in step. The one thing that needs care is picking the track up late — an external file
//! starts at its own 0 — so [`realign_video`] lands it on the playing position when it is selected
//! mid-track. mpv draws it one of two ways, and nothing here knows what it draws into:
//!
//! - through its render API into a GL surface the app owns (a GtkGLArea on Linux,
//!   `src-tauri/src/nativevideo.rs`): the app hands in a GL loader and a wake-up, and calls
//!   [`VideoRenderer::render`] on its GL thread;
//! - with its own video output, into a native window the app hands over
//!   ([`Player::set_video_window`], Windows: `src-tauri/src/nativevideo_windows.rs`).
//!
//! Videos are keyed by the audio URL mpv was handed, never by position in mpv's playlist, so the
//! gapless-next track's video can be registered minutes early and attaches itself the moment that
//! file becomes the one playing.

use std::collections::VecDeque;
use std::ffi::{c_void, CStr};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};

use crate::{quoted, Decks, Error, Player};

/// The playing track and the gapless-next one are all that matter; the rest covers a skip landing
/// while a resolve is in flight.
const MAX_VIDEOS: usize = 4;

#[derive(Default)]
pub(crate) struct Videos {
    /// `(audio URL exactly as mpv was handed it, video URL)`, newest last.
    by_audio: VecDeque<(String, String)>,
    /// The file each deck has open, as of its last `FileLoaded`.
    loaded: [Option<String>; 2],
    /// Counts the files a deck starts — by [`Player::load`], or by mpv advancing to the next
    /// playlist entry on its own. An external video track does not survive its file (`loadfile`
    /// takes the old entry's tracks with it), so this is what says whether a `video-add` still
    /// counts.
    load_seq: [u64; 2],
    /// `(the `load_seq` it was added under, audio URL, video URL)` last handed to `video-add` for
    /// each deck, if any. Both asynchronous entry points — the bounded attach a track start waits
    /// on and the `FileLoaded` that follows it — want to add the same picture, and `video-add` of
    /// one URL twice is a second external track in mpv, not a no-op (mpv 0.41). Cleared when an
    /// add fails, so the retry can run.
    attached: [Option<(u64, String, String)>; 2],
    /// Set by [`Player::load`] when it hands a deck a new file: the `FileLoaded` that follows is
    /// that same file starting, not another one, so it must not count a second time (its picture
    /// is the attach the load's caller makes).
    ours: [bool; 2],
}

impl Videos {
    /// Remember `video` for `audio`, replacing an older entry for the same file.
    fn insert(&mut self, audio: &str, video: &str) {
        self.by_audio.retain(|(a, _)| a != audio);
        if self.by_audio.len() >= MAX_VIDEOS {
            self.by_audio.pop_front();
        }
        self.by_audio.push_back((audio.to_owned(), video.to_owned()));
    }

    fn video_for(&self, deck: usize) -> Option<String> {
        let path = self.loaded[deck].as_deref()?;
        self.by_audio.iter().rev().find(|(a, _)| a == path).map(|(_, v)| v.clone())
    }

    /// [`Player::load`] is about to hand `deck` a new file: a new instance, whose external tracks
    /// do not exist yet.
    fn begin_load(&mut self, deck: usize) {
        self.load_seq[deck] += 1;
        self.ours[deck] = true;
        self.attached[deck] = None;
    }

    /// A file started in `deck`; bumps the sequence unless it was the one `begin_load` announced.
    fn begin_file(&mut self, deck: usize) {
        if !std::mem::take(&mut self.ours[deck]) {
            self.load_seq[deck] += 1;
        }
    }

    /// Record `video` as handed to `video-add` for the file `deck` has open, or answer `false`
    /// when that exact picture is already there for it.
    fn claim_attach(&mut self, deck: usize, audio: &str, url: &str) -> bool {
        let key = (self.load_seq[deck], audio.to_owned(), url.to_owned());
        if self.attached[deck].as_ref() == Some(&key) {
            return false;
        }
        self.attached[deck] = Some(key);
        true
    }

    /// Take back [`Videos::claim_attach`] when the add failed, so a retry can run.
    fn release_attach(&mut self, deck: usize, audio: &str, url: &str) {
        let key = (self.load_seq[deck], audio.to_owned(), url.to_owned());
        if self.attached[deck].as_ref() == Some(&key) {
            self.attached[deck] = None;
        }
    }
}

/// The display connection the app's GL context runs on. mpv needs it for zero-copy hardware
/// decoding (VA-API); without it that falls back to decoding on the CPU, which still plays.
pub enum GlDisplay {
    X11(*const c_void),
    Wayland(*const c_void),
}

impl Player {
    /// `video_url` is the picture for the audio file mpv was (or will be) handed as `audio_url`.
    /// Attached now if that file is the one playing, otherwise when it starts.
    pub fn set_video_for(&self, audio_url: &str, video_url: &str) {
        let deck = self.decks.active.load(Ordering::SeqCst);
        let now = {
            let mut v = self.decks.videos.lock().unwrap();
            v.insert(audio_url, video_url);
            v.loaded[deck].as_deref() == Some(audio_url)
        };
        if now {
            let _ = spawn_video_add(&self.decks, deck, audio_url.to_owned(), video_url.to_owned());
        }
    }

    /// Attach `video_url` as the picture for `audio_url` and wait at most `wait` for mpv to have
    /// the file open. `true` when it did.
    ///
    /// A track start calls this with the deck paused ([`Player::load_paused`]), so a picture that
    /// lands inside `wait` is selected before the first sample and needs no alignment. One that
    /// does not is left to its thread and realigned the moment it opens ([`open_video`]); the
    /// caller plays on either way, so a slow or stalled upstream can no longer hold the start
    /// (field: a proxy `HTTP 504` held it 20 s with `sync video-add failed … Raw(-12)`).
    pub fn attach_video_wait(&self, audio_url: &str, video_url: &str, wait: Duration) -> bool {
        let deck = self.decks.active.load(Ordering::SeqCst);
        {
            let mut v = self.decks.videos.lock().unwrap();
            v.insert(audio_url, video_url);
            v.loaded[deck] = Some(audio_url.to_owned());
        }
        let Some(rx) =
            spawn_video_add(&self.decks, deck, audio_url.to_owned(), video_url.to_owned())
        else {
            // Already attached, or a thread is opening it for this same pair: nothing to wait for.
            return true;
        };
        match rx.recv_timeout(wait) {
            Ok(ok) => ok,
            Err(_) => {
                tracing::debug!(
                    deck,
                    wait_ms = wait.as_millis(),
                    "video: still opening; playback starts without it"
                );
                false
            }
        }
    }

    /// The file mpv is playing, exactly as it was handed it.
    pub fn current_path(&self) -> Option<String> {
        self.mpv().get_property::<String>("path").ok()
    }

    /// The active deck is about to be handed a new file ([`Self::load`]).
    ///
    /// A `loadfile` takes the old entry's external tracks with it — the picture included — so
    /// whatever the video bookkeeping thought was attached is gone, and a picture for the file
    /// starting has to be added again even if the URL is the same one (a replay, or a retry on a
    /// still-live link).
    pub(crate) fn note_load(&self) {
        let deck = self.decks.active.load(Ordering::SeqCst);
        self.decks.videos.lock().unwrap().begin_load(deck);
    }

    /// Whether anyone can see the picture. Off, mpv decodes no video at all; back on, it picks the
    /// track up again at the current position, already in step.
    pub fn set_video_visible(&self, on: bool) {
        if self.decks.video_visible.swap(on, Ordering::SeqCst) != on {
            apply_vid(&self.decks);
        }
    }

    /// Draw the picture into the native window `wid` (mpv's `wid`, an HWND on Windows) with mpv's
    /// own video output, instead of through the render API. Every deck, including the crossfade
    /// deck built later. An error leaves the render API output in place, which with no context
    /// shows nothing: never a window of mpv's own.
    pub fn set_video_window(&self, wid: i64) -> Result<(), Error> {
        for deck in 0..2 {
            if let Some(m) = self.decks.mpv(deck) {
                embed(m, wid)?;
            }
        }
        self.decks.wid.store(wid, Ordering::SeqCst);
        Ok(())
    }

    /// Whether the deck being heard has its video output up, i.e. mpv's window (black until a
    /// frame) is in the one handed to [`Player::set_video_window`].
    pub fn video_output_up(&self) -> bool {
        self.mpv().get_property::<bool>("vo-configured").unwrap_or(false)
    }

    /// A copy of the picture shrunk to at most `max_width` pixels wide, for a glow drawn around it.
    /// `None` with no picture up, or when playback still sits at `unless_at` (paused on the frame
    /// already taken). Costs a full-size conversion inside mpv plus a pass over it here, so it is
    /// for a caller that asks a few times a second, not per frame.
    pub fn video_thumbnail(&self, max_width: usize, unless_at: Option<f64>) -> Option<Thumbnail> {
        let mpv = self.mpv();
        let pts = mpv.get_property::<f64>("time-pos").ok()?;
        // No picture out yet (the video track is still being added): mpv would refuse the
        // screenshot and log an error for every ask.
        if unless_at == Some(pts) || mpv.get_property::<i64>("video-out-params/w").is_err() {
            return None;
        }
        let (width, height, rgba) = screenshot_shrunk(mpv, max_width)?;
        Some(Thumbnail { pts, width, height, rgba })
    }

    /// The renderer, for the app's GL thread. `get_proc_address` resolves GL functions for the
    /// context that thread renders with; `wake` is called from any thread when there is a new frame
    /// to draw (or a new deck to give a context), and must get [`VideoRenderer::render`] called
    /// soon on the GL thread. Creates nothing yet: see [`VideoRenderer::ensure_contexts`].
    pub fn video_renderer(
        &self,
        get_proc_address: fn(&(), &str) -> *mut c_void,
        display: Option<GlDisplay>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> VideoRenderer {
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let w = wake.clone();
        let _ = self.decks.on_new_deck.set(Box::new(move || w()));
        VideoRenderer {
            ctx: [None, None],
            decks: self.decks.clone(),
            get_proc_address,
            display,
            wake,
        }
    }
}

/// `vid` on every deck: on only for the one being heard, and only while someone can see it.
///
/// Under the `videos` lock, like the selection after a `video-add` (see [`add_video`]): the two
/// race, and whichever writes last has to be the one that read the current visibility.
///
/// In a native window (`wid`) mpv's window *is* the picture, and the app's window is see-through
/// around it, so `force-window` follows `vid`: the output stays up, black, between two tracks and
/// while the next one's video is still being added, instead of closing and letting the desktop
/// show through. Off with the picture, which closes the output and frees what it held.
pub(crate) fn apply_vid(decks: &Arc<Decks>) {
    let _serial = decks.videos.lock().unwrap();
    let active = decks.active.load(Ordering::SeqCst);
    let on = decks.video_visible.load(Ordering::SeqCst);
    let embedded = decks.wid.load(Ordering::SeqCst) != 0;
    for deck in 0..2 {
        if let Some(m) = decks.mpv(deck) {
            let show = on && deck == active;
            if embedded {
                let _ = m.set_property("force-window", if show { "yes" } else { "no" });
            }
            if show {
                if let Some(id) = newest_video_track(m) {
                    // A `vid=no` deck that played on — the view was closed, or this is the
                    // incoming deck of a crossfade — starts its picture at the external file's
                    // own 0 when it is selected again, to be raced back to the clock: the
                    // catch-up stutter. Realigning it costs one seek; it is a no-op when the
                    // track was already selected.
                    let fresh = selected_video_track(m) != Some(id);
                    let _ = m.set_property("vid", id);
                    if fresh {
                        realign_video(decks, deck, m);
                    }
                } else {
                    let _ = m.set_property("vid", "auto");
                }
            } else {
                let _ = m.set_property("vid", "no");
            }
        }
    }
}

/// Point one deck at the native window `wid`. The window first: an output switched to before it
/// would open a window of its own. `video-timing-offset` goes back to mpv's default, which the
/// render API setup in `new_mpv` zeroed for its own reasons.
pub(crate) fn embed(mpv: &libmpv2::Mpv, wid: i64) -> Result<(), Error> {
    mpv.set_property("wid", wid)?;
    mpv.set_property("vo", "gpu-next,gpu")?;
    let _ = mpv.set_property("video-timing-offset", 0.05);
    Ok(())
}

/// See [`Player::video_thumbnail`].
pub struct Thumbnail {
    /// The playback position it was taken at, to hand back as `unless_at`.
    pub pts: f64,
    pub width: usize,
    pub height: usize,
    /// RGBA, rows bottom-up (GL's order, as the Linux GL readback produces them).
    pub rgba: Vec<u8>,
}

/// `screenshot-raw` of the decoded frame (no OSD, mpv's default `bgr0`), shrunk with [`shrink`]
/// before mpv's full-size copy is freed.
///
/// Rendered by the video output, so it works on hardware-decoded frames, which mpv's software
/// screenshot refuses (NVDEC answered `MPV_ERROR_COMMAND`). Measured on Linux/NVIDIA, 720p VP9:
/// ~8.5 ms of CPU a grab, and the call returns with mpv's next frame (~40 ms at 24 fps). The
/// software path costs ~6 ms but only with copy-back decoding of every frame, so it is not used.
fn screenshot_shrunk(mpv: &libmpv2::Mpv, max_width: usize) -> Option<(usize, usize, Vec<u8>)> {
    use libmpv2_sys as sys;
    let mut args = [c"screenshot-raw".as_ptr(), c"video".as_ptr(), std::ptr::null()];
    // SAFETY: an all-zero node is MPV_FORMAT_NONE, what mpv expects to fill in.
    let mut node: sys::mpv_node = unsafe { std::mem::zeroed() };
    // SAFETY: a null-terminated list of static strings, on a handle that lives as long as `mpv`.
    if unsafe { sys::mpv_command_ret(mpv.ctx.as_ptr(), args.as_mut_ptr(), &mut node) } < 0 {
        return None;
    }
    let (mut w, mut h, mut stride, mut data) = (0i64, 0i64, 0i64, None);
    if node.format == sys::mpv_format_MPV_FORMAT_NODE_MAP {
        // SAFETY: a NODE_MAP's `list` has `num` keys and values, each read as its own `format`.
        unsafe {
            let list = &*node.u.list;
            for i in 0..list.num.max(0) as usize {
                let key = CStr::from_ptr(*list.keys.add(i)).to_bytes();
                let v = &*list.values.add(i);
                match (key, v.format) {
                    (b"w", sys::mpv_format_MPV_FORMAT_INT64) => w = v.u.int64,
                    (b"h", sys::mpv_format_MPV_FORMAT_INT64) => h = v.u.int64,
                    (b"stride", sys::mpv_format_MPV_FORMAT_INT64) => stride = v.u.int64,
                    (b"data", sys::mpv_format_MPV_FORMAT_BYTE_ARRAY) => {
                        let ba = &*v.u.ba;
                        data = Some(std::slice::from_raw_parts(ba.data as *const u8, ba.size));
                    }
                    _ => {}
                }
            }
        }
    }
    let out = data.and_then(|d| shrink(d, w as usize, h as usize, stride as usize, max_width));
    // SAFETY: `node` came from mpv, and `data` (borrowed from it) is not used past this point.
    unsafe { sys::mpv_free_node_contents(&mut node) };
    out
}

/// A `bgr0` image averaged down by the power of two that brings it to at most `max_width` wide
/// (the same halving the Linux GPU chain does, so the glow looks the same), as RGBA rows
/// bottom-up. Every source pixel counts: a sparse sample of a frame shimmers.
fn shrink(
    src: &[u8],
    w: usize,
    h: usize,
    stride: usize,
    max_width: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let mut f = 1;
    while w / f > max_width.max(1) {
        f *= 2;
    }
    let (lw, lh) = (w / f, h / f);
    if lw == 0 || lh == 0 || stride < w * 4 || src.len() < stride * (h - 1) + w * 4 {
        return None;
    }
    let n = (f * f) as u32;
    let mut out = vec![0u8; lw * lh * 4];
    for oy in 0..lh {
        let dst_row = (lh - 1 - oy) * lw * 4;
        for ox in 0..lw {
            let mut acc = [0u32; 3];
            for y in oy * f..(oy + 1) * f {
                let at = y * stride + ox * f * 4;
                for px in src[at..at + f * 4].chunks_exact(4) {
                    acc[0] += px[2] as u32;
                    acc[1] += px[1] as u32;
                    acc[2] += px[0] as u32;
                }
            }
            let o = dst_row + ox * 4;
            out[o..o + 4].copy_from_slice(&[
                (acc[0] / n) as u8,
                (acc[1] / n) as u8,
                (acc[2] / n) as u8,
                255,
            ]);
        }
    }
    Some((lw, lh, out))
}

/// A deck finished opening a file. Reads which one on its own thread: the caller is the event
/// loop, and a synchronous property read there can stall the pump mid-transition.
pub(crate) fn file_loaded(decks: &Arc<Decks>, deck: usize) {
    let Some(mpv) = decks.mpv(deck).cloned() else { return };
    let decks = decks.clone();
    let _ = std::thread::Builder::new().name("mpv-video".into()).spawn(move || {
        let path = mpv.get_property::<String>("path").ok();
        let video = {
            let mut v = decks.videos.lock().unwrap();
            v.loaded[deck] = path.clone();
            v.begin_file(deck);
            (deck == decks.active.load(Ordering::SeqCst)).then(|| v.video_for(deck)).flatten()
        };
        if let (Some(audio), Some(url)) = (path, video) {
            let _ = spawn_video_add(&decks, deck, audio, url);
        }
    });
}

/// A crossfade made the other deck the one being heard: move the picture over with the sound.
pub(crate) fn deck_swapped(decks: &Arc<Decks>) {
    apply_vid(decks);
    let deck = decks.active.load(Ordering::SeqCst);
    let (audio, video) = {
        let v = decks.videos.lock().unwrap();
        (v.loaded[deck].clone(), v.video_for(deck))
    };
    if let (Some(audio), Some(url)) = (audio, video) {
        let _ = spawn_video_add(decks, deck, audio, url);
    }
}

/// [`Player::attach_video_wait`]'s worker: `video-add` on a thread of its own, because the
/// command returns only once mpv has opened the file, which is a network round trip. The receiver
/// reports whether the picture opened, so a caller can stop waiting on it. `None` when this exact
/// pair is already attached (or still opening) for the deck — see [`Videos::attached`].
fn spawn_video_add(
    decks: &Arc<Decks>,
    deck: usize,
    audio: String,
    url: String,
) -> Option<Receiver<bool>> {
    if !decks.videos.lock().unwrap().claim_attach(deck, &audio, &url) {
        tracing::debug!(deck, "video: already attached for this file, not adding it twice");
        return None;
    }
    let Some(mpv) = decks.mpv(deck).cloned() else {
        decks.videos.lock().unwrap().release_attach(deck, &audio, &url);
        return None;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let decks = decks.clone();
    let _ = std::thread::Builder::new().name("mpv-video-add".into()).spawn(move || {
        let ok = open_video(&decks, deck, &mpv, &audio, &url);
        if !ok {
            decks.videos.lock().unwrap().release_attach(deck, &audio, &url);
        }
        let _ = tx.send(ok);
    });
    Some(rx)
}

/// One `video-add`, and the selection and alignment that have to follow it. `true` when the file
/// opened.
///
/// Always added unselected, and selected afterwards by id if someone can see it by then. Choosing
/// `select` or `auto` up front read the visibility before the round trip: the view asking for the
/// picture during it set `vid=auto` while there was no track to pick, the track then arrived
/// unselected, and setting `auto` again is a no-op in mpv. That was the black box after a track
/// change or an opened link, which only closing and reopening the view (`no` then `auto`) cleared.
///
/// `audio` is the file the picture belongs to, which a failure names.
fn open_video(
    decks: &Arc<Decks>,
    deck: usize,
    mpv: &Arc<libmpv2::Mpv>,
    audio: &str,
    url: &str,
) -> bool {
    if let Err(e) = add_external_video(mpv, url) {
        tracing::warn!(deck, error = %e, "video: mpv could not open the picture");
        let _ = decks.tx.send(crate::PlayerEvent::VideoFailed(audio.to_owned()));
        return false;
    }
    let _serial = decks.videos.lock().unwrap();
    let shown = decks.video_visible.load(Ordering::SeqCst)
        && decks.active.load(Ordering::SeqCst) == deck;
    let track = shown.then(|| newest_video_track(mpv)).flatten();
    let _ = match track {
        Some(id) => mpv.set_property("vid", id),
        None => mpv.set_property("vid", "no"),
    };
    if track.is_some() {
        realign_video(decks, deck, mpv);
    }
    tracing::debug!(deck, ?track, "video: attached");
    true
}

/// `video-add`, retried while mpv refuses it.
///
/// This runs in the same breath as the `loadfile` that starts the file it belongs to, and for the
/// few milliseconds in which the entry being replaced is gone and the new one has not started, mpv
/// has no current file to add an external track to and answers `MPV_ERROR_COMMAND` — the field's
/// `Raw(-12)`, which read as a failed attach. A retry 20 ms later already succeeds (mpv 0.41), so
/// wait out the replacement instead of losing the picture; the total is short enough to stay
/// inside [`Player::attach_video_wait`]'s budget.
fn add_external_video(mpv: &libmpv2::Mpv, url: &str) -> Result<(), Error> {
    let mut delay = std::time::Duration::from_millis(20);
    let mut last = None;
    for attempt in 0..5 {
        match mpv.command("video-add", &[&quoted(url), "auto"]) {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        if attempt < 4 {
            std::thread::sleep(delay);
            delay *= 2;
        }
    }
    Err(Error::Mpv(last.expect("the loop body runs at least once")))
}

/// Land a picture that was picked up after the audio clock had already started on the position
/// the deck is at, instead of letting mpv read the external file from its own 0 and race it to
/// the clock — the stutter a video switch is reported as, and after which a seek can arrive
/// mid-GOP and fail in OpenH264.
///
/// Detached: a caller may be the event loop (`apply_vid` runs there on a deck swap), and a seek
/// is mpv's work. A paused deck or one at the top needs none, so the mid-track attach and the
/// crossfade hand-over are the only callers it has any effect for.
fn realign_video(decks: &Arc<Decks>, deck: usize, mpv: &Arc<libmpv2::Mpv>) {
    let (decks, mpv) = (decks.clone(), mpv.clone());
    let _ = std::thread::Builder::new().name("mpv-video-align".into()).spawn(move || {
        if decks.active.load(Ordering::SeqCst) != deck
            || mpv.get_property::<bool>("pause").unwrap_or(true)
        {
            return;
        }
        let Ok(pos) = mpv.get_property::<f64>("time-pos") else { return };
        if !pos.is_finite() || pos <= 1.0 {
            return;
        }
        match mpv.command("seek", &[&pos.to_string(), "absolute+exact"]) {
            Ok(()) => {
                tracing::debug!(deck, pos, "video: aligned picture with the playing position")
            }
            Err(e) => tracing::debug!(deck, pos, error = %e, "video: align seek failed"),
        }
    });
}

/// The id of the video track mpv currently has selected, if any.
fn selected_video_track(mpv: &libmpv2::Mpv) -> Option<i64> {
    let n = mpv.get_property::<i64>("track-list/count").ok()?;
    (0..n).find_map(|i| {
        if mpv.get_property::<String>(&format!("track-list/{i}/type")).ok()?.as_str() != "video" {
            return None;
        }
        if !mpv.get_property::<bool>(&format!("track-list/{i}/selected")).unwrap_or(false) {
            return None;
        }
        mpv.get_property::<i64>(&format!("track-list/{i}/id")).ok()
    })
}

/// The id of the last video track in the file, which is the one `video-add` just appended.
fn newest_video_track(mpv: &libmpv2::Mpv) -> Option<i64> {
    let n = mpv.get_property::<i64>("track-list/count").ok()?;
    (0..n).rev().find_map(|i| {
        let kind = mpv.get_property::<String>(&format!("track-list/{i}/type")).ok()?;
        if kind != "video" {
            return None;
        }
        mpv.get_property::<i64>(&format!("track-list/{i}/id")).ok()
    })
}

/// mpv's render contexts, one per deck, living on the app's GL thread.
pub struct VideoRenderer {
    // Before `decks`: fields drop in order, and mpv requires every render context to be freed
    // before its core is destroyed, which dropping the last `Decks` would do.
    ctx: [Option<RenderContext>; 2],
    decks: Arc<Decks>,
    get_proc_address: fn(&(), &str) -> *mut c_void,
    display: Option<GlDisplay>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl VideoRenderer {
    /// Give every deck that exists a render context. On the GL thread, with its context current,
    /// and before any picture is asked for: a deck with no context cannot start its video output.
    pub fn ensure_contexts(&mut self) -> Result<(), Error> {
        for deck in 0..2 {
            if self.ctx[deck].is_some() {
                continue;
            }
            let Some(mpv) = self.decks.mpv(deck).cloned() else { continue };
            let mut params = vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: self.get_proc_address,
                    ctx: (),
                }),
            ];
            match self.display {
                Some(GlDisplay::X11(p)) => params.push(RenderParam::X11Display(p)),
                Some(GlDisplay::Wayland(p)) => params.push(RenderParam::WaylandDisplay(p)),
                None => {}
            }
            // SAFETY: the handle is live for as long as `self.decks` is, and the field order above
            // frees this context first.
            let handle = unsafe { &mut *mpv.ctx.as_ptr() };
            let mut ctx = RenderContext::new(handle, params)?;
            let wake = self.wake.clone();
            ctx.set_update_callback(move || wake());
            self.ctx[deck] = Some(ctx);
        }
        Ok(())
    }

    /// Draw the playing deck's current frame into `fbo` (`width` x `height` pixels). On the GL
    /// thread, with its context current.
    pub fn render(&mut self, fbo: i32, width: i32, height: i32) -> Result<(), Error> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        self.ensure_contexts()?;
        let deck = self.decks.active.load(Ordering::SeqCst);
        if let Some(ctx) = &self.ctx[deck] {
            // Flipped: a GL framebuffer's origin is bottom-left, a video frame's top-left.
            ctx.render::<()>(fbo, width, height, true)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A video belongs to the file a deck has open, by exact URL, and a re-resolve replaces it.
    #[test]
    fn a_video_follows_its_audio_file() {
        let mut v = Videos::default();
        v.insert("a1", "v1");
        v.insert("a2", "v2");
        assert_eq!(v.video_for(0), None, "nothing loaded yet");
        v.loaded[0] = Some("a2".into());
        v.loaded[1] = Some("a1".into());
        assert_eq!(v.video_for(0).as_deref(), Some("v2"));
        assert_eq!(v.video_for(1).as_deref(), Some("v1"));
        v.insert("a2", "v2b");
        assert_eq!(v.video_for(0).as_deref(), Some("v2b"));
        assert_eq!(v.by_audio.len(), 2);
        for i in 0..MAX_VIDEOS {
            v.insert(&format!("x{i}"), "x");
        }
        assert_eq!(v.by_audio.len(), MAX_VIDEOS);
        assert_eq!(v.video_for(0), None, "evicted by newer tracks");
    }

    /// One `video-add` per file. Both entry points — the bounded attach a track start waits on and
    /// the `FileLoaded` that follows it — want the same picture, and a replay of the same cached
    /// URLs wants it again (the `loadfile` took the old entry's external tracks with it). mpv 0.41
    /// answers a second `video-add` of one URL with a second track rather than a no-op, so getting
    /// either of these wrong shows up as a doubled picture or no picture at all.
    #[test]
    fn a_picture_is_added_once_per_file_and_again_per_replay() {
        let mut v = Videos::default();
        // A track start: the load, the attach made while it is paused, then its own FileLoaded.
        v.begin_load(0);
        assert!(v.claim_attach(0, "a1", "v1"), "the start's attach adds the picture");
        v.begin_file(0);
        assert!(
            !v.claim_attach(0, "a1", "v1"),
            "the FileLoaded behind the start does not add the same picture twice"
        );
        // An add that failed is taken back, so its retry runs.
        v.release_attach(0, "a1", "v1");
        assert!(v.claim_attach(0, "a1", "v1"), "a retry after a failed add runs");

        // The same track again, same cached URLs: a new file, so its picture is added again.
        v.begin_load(0);
        assert!(v.claim_attach(0, "a1", "v1"), "a replay adds its picture again");
        v.begin_file(0);
        assert!(!v.claim_attach(0, "a1", "v1"), "and its FileLoaded does not duplicate it");

        // mpv advancing on its own: no `begin_load`, so its FileLoaded counts as a new file and the
        // picture claimed for the previous one no longer applies.
        v.begin_file(0);
        assert!(v.claim_attach(0, "a2", "v2"), "the next entry gets its own picture");
        assert!(v.claim_attach(0, "a2", "v2b"), "a re-resolved URL is a new picture");
    }

    /// bgr0 in, RGBA out, each output pixel the mean of its block, rows flipped to bottom-up, and
    /// a padded stride skipped rather than read as pixels.
    #[test]
    fn shrink_averages_swaps_and_flips() {
        // 4x4, stride 20 (a 4-byte pad per row, filled with junk). Top half blue-ish, bottom red.
        let px = |b: u8, g: u8, r: u8| [b, g, r, 0];
        let mut src = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let p = if y < 2 { px(200 + x as u8, 10, 0) } else { px(0, 20, 100) };
                src.extend_from_slice(&p);
            }
            src.extend_from_slice(&[99; 4]);
        }
        let (w, h, out) = shrink(&src, 4, 4, 20, 2).unwrap();
        assert_eq!((w, h), (2, 2));
        // Bottom-up: the first row out is the source's bottom (red) half.
        assert_eq!(&out[0..8], &[100, 20, 0, 255, 100, 20, 0, 255]);
        // Top row: blue averages 200,201 then 202,203 (integer mean).
        assert_eq!(&out[8..16], &[0, 10, 200, 255, 0, 10, 202, 255]);
        // Already narrow enough: copied through, just swapped and flipped.
        let (w, h, _) = shrink(&src, 4, 4, 20, 128).unwrap();
        assert_eq!((w, h), (4, 4));
        assert!(shrink(&src, 4, 4, 12, 2).is_none(), "a stride shorter than a row is refused");
        assert!(shrink(&src[..40], 4, 4, 20, 2).is_none(), "a short buffer is refused");
    }
}
