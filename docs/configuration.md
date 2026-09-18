# OpenPresenter Configuration & Preferences

OpenPresenter provides a global Preferences window (`Cmd+,` on macOS, `Ctrl+,` on Windows/Linux, or by clicking the **Gear icon** in the top navbar or bottom bar).

---

## 1. Preferences Window Overview

The Preferences window is organized into 5 dedicated tabs:

### 1. General
- **Startup Workspace**: Choose between **Show (Live Presentation)** and **Edit (Slide Editor)** mode upon launching.
- **Database Storage**: View current SQLite database location (`library.db`), browse for custom database path, or reveal the database in macOS Finder / system file manager.

### 2. Screens & Outputs
- **Screens & Matrix**: Open the full Looks & Screens Matrix modal directly from preferences.
- **Display Configurations**:
  - Add and manage Window Outputs for external HDMI/DisplayPort projectors and confidence monitors.
  - Add and manage NDI broadcast outputs.
  - Set output resolutions (1080p, 720p, etc.), positions, and alpha transparency modes.
  - Active toggle switches to spawn or close individual physical output windows.

### 3. Network & NDI
- **NDI Stream Settings**:
  - Global NDI Source Name broadcasted across your local network (e.g. `OpenPresenter`).
  - Broadcast Frame Rate selection (**60 FPS** smooth motion or **30 FPS** bandwidth-saving).
- **Remote Triggers & Automation**:
  - **HTTP REST API Port**: Default `9090`. Configure endpoint listener for web/companion triggers.
  - **OSC UDP Port**: Default `9000`. Configure Open Sound Control listener.
  - Direct reference for supported REST endpoints (`/action/next`, `/action/prev`, `/action/clear`, `/action/slide/:index`) and OSC paths.

### 4. Transitions
- **Default Slide Transition**: Select global transition style:
  - **Cut**: Instant cut between slides.
  - **Fade**: Smooth crossfade.
  - **Push Left / Push Right**: Directional push transition.
  - **Wipe Left / Wipe Right**: Directional angular wipe.
  - **Zoom**: Scale zoom transition.
- **Transition Duration**: Slider with real-time feedback (50 ms – 2000 ms in 25 ms steps).

### 5. Advanced
- **Accessibility & Performance**:
  - **Reduce Motion**: Disables decorative animations for high-performance or low-latency rigs.
  - **Default Slide Grid Zoom**: Configures thumbnail column density (2 to 6 columns).
- **Configuration File Management**:
  - Direct path to `config.toml`.
  - Reveal configuration file in Finder / Explorer.
  - **Reset to Defaults**: Restore all configuration options back to factory defaults.
  - System architecture & version diagnostics readout.

---

## 2. Storage Locations & Config File

OpenPresenter automatically initializes configuration and database files on first run in platform-standard directories:

| OS | Configuration Directory | Database File | Config File |
| :--- | :--- | :--- | :--- |
| **macOS** | `~/Library/Application Support/openpresenter/` | `library.db` | `config.toml` |
| **Windows** | `%APPDATA%\openpresenter\` | `library.db` | `config.toml` |
| **Linux** | `~/.config/openpresenter/` | `library.db` | `config.toml` |

---

## 3. Configuration File Specification (`config.toml`)

```toml
[general]
startup_mode = "show" # "show" or "edit"

[output]
# Default output dimensions
width = 1920
height = 1080
# Secondary monitor positioning
screen_x = 1920.0
screen_y = 0.0
# Automatically fullscreen secondary monitor on startup
auto_fullscreen = false

[ndi]
# NDI broadcast stream name visible on the network
source_name = "OpenPresenter"
fps = 60

[trigger]
# REST API trigger port
http_port = 9090
# Open Sound Control UDP listener port
osc_port = 9000

[transition]
type = "fade" # "cut", "fade", "push", "wipe", "zoom"
duration_ms = 400

[ui]
# Enable to suppress decorative motion animations
reduce_motion = false
default_grid_cols = 4
```

---

## 4. Keyboard Shortcuts & Dismissal

- **Open Preferences**: `Cmd+,` (macOS) / `Ctrl+,` (Windows/Linux) or click the Gear icon.
- **Close Preferences**: Press `Escape`, click the `×` button, or click anywhere on the dark backdrop outside the preferences card.
- **Save Changes**: Changes are automatically validated and written immediately to `config.toml`, with an instant visual status notification badge.

