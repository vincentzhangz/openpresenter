# OpenPresenter User Guide

This guide covers the core features, workflow, and operations for OpenPresenter — a live presentation application built in Rust for houses of worship, live production, and broadcast.

---

## 1. Interface Overview

OpenPresenter uses a unified, dark-themed three-dock layout designed for rapid live operation and distraction-free editing.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Top Navigation Bar: Live / Edit Mode · NDI Toggle · Stage · Looks · Record  │
├───────────────┬─────────────────────────────────────────────┬───────────────┤
│ Left Rail     │ Center Workspace                            │ Right Dock    │
│               │                                             │               │
│ • Library     │ • Slide Grid (Live mode)                    │ • Inspector   │
│ • Playlists   │   OR                                        │   - Shape     │
│ • Songs       │ • Slide Canvas (Edit mode)                  │   - Text      │
│ • Bible       │                                             │   - Build     │
│               │ • Bottom Object Chip Strip                  │   - Theme     │
├───────────────┴─────────────────────────────────────────────┴───────────────┤
│ Bottom Bar: Transition duration · View mode switcher · Output settings gear │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Modes: Live vs. Edit

You can switch between **Live** (Show) mode and **Edit** mode using the switcher in the top navigation bar or the bottom bar:

- **Live Mode**:
  - Displays the presentation slide grid or table view for the live operator.
  - **Grid vs. Table View**: Use the view toggle buttons (`table-cells` / `list-ul`) next to the zoom slider in the bottom bar to switch between the visual thumbnail grid and the text-focused sequential table view.
  - Clicking a slide triggers it to live output; right-clicking summons the slide context menu.
  - **Keyboard Navigation**:
    - Advance with `→`, `↓`, `Space`, or `Page Down`; return with `←`, `↑`, or `Page Up`.
    - Jump directly to slides with number keys `1`–`9`.
    - Jump to groups with `V` (Verse) and `C` (Chorus).
    - Clear layers using `F1`–`F6` or `⌘1`–`⌘6` (`Ctrl+1`–`Ctrl+6`).
    - Toggle blackout with `B`.
  - Right dock displays preview (Audience / Stream / Stage), quick show controls, active timers, props, and layer clear buttons.
- **Edit Mode**:
  - Displays the full canvas slide editor with zoom and pan.
  - Bottom strip provides interactive chips for each layer/object on the active slide.
  - Right dock displays the four-tab Inspector (**Shape/Layers**, **Text**, **Build/Slide**, and **Theme**).

---

## 3. Slide & Layer Editing

### Adding and Managing Layers
In the **Shape** tab of the Inspector (or using the canvas toolbar):
- Click **+ Text** to add a text layer.
- Click **+ Rect**, **+ Ellipse**, or **+ Line** to add vector shape overlays.
- **Layer List**:
  - **Visibility**: Click the eye icon (`eye` / `eye-slash`) on any layer row to show or hide that object on output.
  - **Lock**: Click the lock icon (`lock` / `lock-open`) to lock an object against accidental movement.
  - **Reordering**: Use the up and down arrow buttons to adjust z-order.
  - **Delete**: Click the red `xmark` button on a row to delete that layer.
  - **Bottom Object Chip Strip**: Click any object chip at the bottom of the canvas to select that layer for instant property editing.

### Typography & Text Styling
In the **Text** tab of the Inspector:
- **Style Clipboard**:
  - Click **Copy Style** to copy the current font, weight, colors, stroke, and effects.
  - Select another object or slide and click **Paste Style** to apply the style instantly.
- **Font Family**: Click the font family dropdown button to cycle through standard presentation typefaces (`Arial`, `Inter`, `Helvetica`, `Georgia`, `Impact`, `Roboto`, `Times New Roman`).
- **Font Weight**: Click **Bold** / **Regular** to toggle bold weight.
- **Font Size**: Type a custom pt size in the input field, or click `-` and `+` to step the size down or up.
- **Capitalization**: Click the capitalization selector to cycle between `None`, `All Caps`, `Lower Case`, and `Title Case`.
- **Color Presets**:
  - Click the color swatch or palette icon to cycle through presentation-grade color presets.
  - Click any of the 8 quick color swatches below to apply White, Pale Yellow, Gold, Coral, Sky Blue, Mint, Silver, or Charcoal.
- **Alignment & Position**: Click left, center, or right alignment; use the arrow buttons to position text at the top, center, or bottom of the slide.
- **Effects**: Toggle **Stroke**, **Shadow**, or **Glow Effect** checkboxes.

---

## 4. Presentations & Service Playlists

### Presentations
- **Create**: Click **New Presentation** in the left rail to create a presentation document.
- **Import Package (`.opp`)**: Click **Import .opp** in the Presentations list to import an OpenPresenter zip package containing slides and bundled media.
- **Export Package (`.opp`)**: Click **Export .opp** to package the currently open presentation and its referenced assets into a portable bundle.

### Service Playlists
- Select the **Playlists** tab in the left rail to organize Sunday services or event sets.
- Order presentations, songs, and scripture readings into a sequential rundown.
- Reorder items via drag-and-drop or item controls.

---

## 5. Songs & Lyrics

- Select the **Songs** tab in the left rail to search your lyric database.
- **Verse Structure**: Organize songs by sections (`Verse 1`, `Chorus 1`, `Verse 2`, `Bridge`, etc.).
- **Send to Presentation**: Click **Send to Presentation** in the song editor to compile song verses into an editable presentation document ready for live display.
- **OpenLyrics XML**:
  - Click **Import OpenLyrics** in the songs list to import standard church presentation XML lyrics.
  - Click **Export OpenLyrics** in the song editor toolbar to export lyrics in standard OpenLyrics XML.

---

## 6. Scripture & Bible

- Select the **Bible** tab in the left rail.
- Search verses using full-text search (FTS5) by book, chapter, and verse range.
- Select scripture verses and send them directly to live output or add them as slides to the current service plan.

---

## 7. Multi-Screen Outputs & Looks Matrix

### Screen Configuration
Click the gear icon in the bottom bar or **Configure Screens** inside the Looks Matrix:
- Add **Window** outputs for secondary monitors or physical projectors.
- Add **NDI** outputs for streaming to OBS, vMix, or broadcast switchers over local network.
- Toggle outputs active/inactive, switch resolutions (720p / 1080p), and select content routing.

### Looks Matrix
Click **Looks** in the top navigation bar to open the Looks & Screens Matrix modal:
- Create and save named **Looks** (e.g. *Worship*, *Sermon*, *Livestream*, *Announcements*).
- Route or suppress individual layers (Props, Slides, Backgrounds) per physical output or NDI feed.
- Apply different slide themes to audience screens vs. confidence/stream monitors from a single live trigger.

---

## 8. Live Outputs & Streaming

### NDI Output
- Click **NDI** in the top navigation bar to start or stop the real-time 30 fps NDI output stream.
- The badge turns green with a checkmark when broadcasting, and returns to normal when stopped.

### Recording
- Click **Record** in the top navigation bar to open the recording panel.
- Records output video to high-efficiency H.264 video files with bounded backpressure.
