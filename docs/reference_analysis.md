# The reference analysis

This is what the reverse-engineering of the macOS build produced, and how
the app was built from it. It is here so the next change can be checked
against the same evidence rather than against a guess.

> The reference application is not named anywhere in this repository. The
> analysis below is of a third-party macOS presentation application, and
> only its *shape* — the panels, the commands and the wording — is used.

## How it was read

The build was analysed with the `rea-agents` CLI, which decodes compiled
Interface Builder archives and reads the application's own localisation
files. Three artefacts carried the design:

| Artefact | What it gave |
| --- | --- |
| 85 compiled `.nib` archives | The exact panel classes, the view hierarchy, the outlets, the actions and the frames |
| 6,666-node graph of those nibs | The connections between the controls and the code that drives them |
| 5,596-key localisation file | Every operator-facing string, which is the app's own list of its features |

The decoded nibs are the ground truth for *where* a control lives; the
localisation file is the ground truth for *what it is called*. Together they
are a specification, which is why the menu in this app is a transcription
rather than an interpretation.

## What the reference's editor is made of

- **A menu bar** with the app's own menu, File, Edit, Editor, Presentation,
  View, Window and Help.
- **A three-dock window**: a left rail (presentations, playlists, library,
  songs, scripture), a centre (the canvas and the slide list) and a right
  dock (the inspector and the show controls).
- **A bottom object toolbar** in the editor, holding everything that can go
  on a slide.
- **A show header** with a presentation timeline: the slides as blocks, the
  cues on them, a playhead and a transport.
- **A clears-and-layers vocabulary** shared by the dock, the clear groups
  and the menu.

## The commands, and where they ended up

| Reference command | In this app |
| --- | --- |
| Editor ▸ Insert | `src/ui/insert.rs`, the menu and the object toolbar under the canvas |
| Editor ▸ Arrangement | `src/domain/mod.rs` — align, draw order, mirror, lock |
| Editor ▸ Text | `src/domain/mod.rs` — the style carries the casing, baseline, ligatures, underline and strike |
| Edit ▸ Transformations | `src/domain/mod.rs` — `TextRewrite`, which rewrites the words themselves |
| Editor ▸ Zoom | `src/ui/main_window.rs` — the canvas fits, zooms and shows the rulers and guides |
| Presentation ▸ Clear * | `src/ui/clear_groups.rs` — named sets of the output's layers |
| Presentation ▸ Clear Announcements | `src/domain/props.rs` — the announcement is its own layer |
| File ▸ Import ▸ PowerPoint * | `src/import/pptx.rs` — the file's own XML, in three forms |
| View ▸ Test Patterns | `src/render/test_pattern.rs` — drawn over the output |
| View ▸ Timeline | `src/ui/timeline.rs` — the show as a piece of time |
| Application ▸ Settings | `src/ui/settings.rs` — thirteen pages, each with settings that are really wired |

## The macOS version

The reference is a **macOS 14.6** application. That is the floor its
`Info.plist` declares and the floor its Mach-O carries:

```
$ plutil -extract LSMinimumSystemVersion raw Info.plist
14.6
$ otool -l <the reference's executable> | grep -A 5 LC_BUILD_VERSION
    minos 14.6
```

This app's floor is **macOS 12.0**, and that is what its bundle declares.
The difference is deliberate: the reference uses framework APIs that arrived
with Sequoia, and this app uses none of them. The Mach-O is built against a
floor of 11.0, and the bundle asks for 12.0 — a version with the windowing
and graphics features the app actually uses.

The SDK is the same on both sides (`sdk 27.0`), so the two are built with
one toolchain and differ only in what they ask the operating system for.

> The reference's own version numbers are `CFBundleShortVersionString 22`
> and `CFBundleVersion 369098899`. Neither is carried over: this app's
> version is its own.

## The rules the analysis produced

Three of them are now in `AGENTS.md` as the app's own rules, because they are
the ones that keep the two paths agreeing:

- **The editor's mirrors are a second copy of the model.** Every path that
  changes the slide's text has to update the mirror.
- **A value the model clamps is shown clamped.** One rule for the font
  size, so the two panels cannot disagree.
- **A property the renderer honours and the editor cannot reach is a
  property nobody can use.** So the text style, the shape and the
  arrangement all have an editor as well as an output.

## The differences that are deliberate

- **The cloud is not here.** The reference's account, subscription and
  content-download features are not implemented, and the settings pages for
  them are not in the menu.
- **The reference's own encodings are not here.** H.264 and H.265 are, and
  the capture presets carry the choice; the reference's ProRes, HAP and
  hardware encoders are not.
- **The broadcast destinations are local.** The reference can send to RTMP
  and to a hosted service; this app sends to the network over NDI and to
  disk.
- **Device protocols are not here.** The reference also speaks DMX/Art-Net,
  serial devices, Sony camera sync and Resi/Syphon capture assistants; the
  network control surface here is HTTP, OSC and MIDI only, and NDI is the
  only broadcast destination.
- **The reference's spelling of the group names is kept** — Verse 1–6,
  Chorus 1–4, Bridge 1–3, PreChorus, Tag, Turnaround, Vamp, Intro, Outro,
  Ending, Interlude — because the colours are keyed off them.
- **Two of the reference's imports are not here.** Its *MultiTracks* import
  needs an authorisation from the vendor's servers, and its *SongSelect by
  CCLI* import looks the words up on a network service. Both are licensed
  services rather than formats, so neither is implemented; the songs this
  app reads come in as OpenLyrics XML.
- **The reference's PowerPoint imports are here, in three forms.** Its menu
  names them *PowerPoint as Objects and Text…*, *PowerPoint as Images…* and
  *PowerPoint as Presentation…*, and this app uses the same three names.
  What is different is how the file is read: the reference drives
  Microsoft PowerPoint itself, so it needs PowerPoint 2016 or later installed
  on the machine doing the import, and it draws its pictures by exporting
  the deck to PDF first. This app reads the file's own XML, so it needs
  nothing installed and it keeps the text, the pictures, the clips, the
  backgrounds, the colours and the transitions. The one thing neither keeps
  is the animation — the reference's own words for its native importer
  promise "slide elements, media, transitions, and builds", and the builds
  are the one part this app does not carry.
- **The reference's fourth PowerPoint entry is not here.** Its *PowerPoint…*
  item opens the older options sheet — as JPEG images, as text objects, as
  text and graphic objects, with or without the slide backgrounds — and the
  reference's own strings carry a *Not Implemented* error for that path. The
  three entries this app has cover the outcomes that sheet offered, so a
  second way to ask the same question is not a feature anybody can use.
- **The library is one library.** The reference's *New Library* command
  creates a new library file and switches to it; this app keeps a single
  database at `config.db_path` (under the platform's Application Support)
  for every repository, and the file menu's *New Library* opens the Library
  rail where media import lives. Creating a second database would require
  re-pointing every repository and the queue bridge at a new connection
  mid-session, which is a feature, not a bug fix — it is not faked.
- **Timecode is the stage clock.** The reference's *View ▸ Timecode* shows
  the stage clock; this app's stage display already shows the operator's
  local time, the countdown and the elapsed timer, so the menu's Timecode
  item opens the same surface instead of a second clock.
- **Continuous Playlist loops the plan, not the slides.** The reference's
  continuous playlist loops the service; this app does the same thing in
  the playlist transport (`Continuous ✓` in the panel, checked in the View
  menu) and with the timeline's loop flag.

## How to check the next change

1. The reference's own wording is in the localisation file; the app's
   wording in `src/ui/menus.rs` and in the panels. A command that exists in
   one and not the other is a gap.
2. Every command in the menu must have a handler. The check is mechanical:

   ```bash
   python3 - <<'PY'
   import re
   menus = open('src/ui/menus.rs').read()
   actions = open('src/ui/actions.rs').read()
   main_window = open('src/ui/main_window.rs').read()
   menu_actions = set(re.findall(r'actions::([A-Za-z0-9_]+)', menus)) - {'menu_actions'}
   route_actions = set(
       re.findall(
           r'Box::new\(([A-Za-z0-9_]+)\)',
           actions.split('pub(crate) fn menu_actions()')[-1].split('\n///')[0],
       )
   )
   body = main_window[
       main_window.index('pub(crate) fn handle_action'):main_window.index('fn action_route(')
   ]
   handled_actions = set(re.findall(r'"([A-Za-z0-9_]+)"', body))
   print('menu actions missing a route:', sorted(menu_actions - route_actions))
   print('menu actions missing a handler:', sorted(menu_actions - handled_actions))
   PY
   ```

   An empty list means every command in the menu does something. The check
   compares whole names: a substring test would let `ImportPowerPoint` look
   handled because `ImportPowerPointAsImages` is there.
3. The menu is rebuilt from the state it shows, so a check mark in the menu
   is the truth about the slide.
