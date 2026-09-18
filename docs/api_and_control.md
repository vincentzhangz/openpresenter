# OpenPresenter API & Remote Control Reference

OpenPresenter features built-in HTTP and OSC (Open Sound Control) trigger listeners, allowing seamless integration with Stream Deck, Companion, QLab, broadcast automation systems, and hardware switchers.

---

## 1. HTTP REST API

The HTTP listener runs by default on port `9090` (configurable in `config.toml` under `[http_trigger]`).

### Endpoints

| Method | Endpoint | Description | Example Request |
| :--- | :--- | :--- | :--- |
| `GET` | `/api/status` | Application health & status check | `curl http://localhost:9090/api/status` |
| `POST` | `/api/slides/next` | Advance to the next slide | `curl -X POST http://localhost:9090/api/slides/next` |
| `POST` | `/api/slides/prev` | Step back to previous slide | `curl -X POST http://localhost:9090/api/slides/prev` |
| `POST` | `/api/slides/goto/{idx}` | Jump to slide by 0-indexed number | `curl -X POST http://localhost:9090/api/slides/goto/2` |
| `POST` | `/api/black/{on}` | Toggle black screen mode (`true` or `false`) | `curl -X POST http://localhost:9090/api/black/true` |
| `POST` | `/api/clear` | Clear output to background | `curl -X POST http://localhost:9090/api/clear` |
| `POST` | `/api/timer/start` | Start the stage / countdown timer | `curl -X POST http://localhost:9090/api/timer/start` |
| `POST` | `/api/timer/stop` | Stop the stage / countdown timer | `curl -X POST http://localhost:9090/api/timer/stop` |
| `POST` | `/api/timer/reset` | Reset timer to zero / start epoch | `curl -X POST http://localhost:9090/api/timer/reset` |

---

## 2. OSC (Open Sound Control) Protocol

The OSC listener binds by default to UDP port `9000` (configurable in `config.toml` under `[osc_trigger]`).

### Address Map

| OSC Address | Arguments | Behavior |
| :--- | :--- | :--- |
| `/slide/next` | None | Advance to next slide in presentation |
| `/slide/prev` | None | Return to previous slide |
| `/slide/goto` | `int32` (index) | Trigger specific slide index |
| `/black` | `bool` or `int32` | Enable (`true` / `1`) or disable (`false` / `0`) black screen |
| `/clear` | None | Clear active slide layers |
| `/timer/start` | None | Start the stage timer |
| `/timer/stop` | None | Stop the stage timer |
| `/timer/reset` | None | Reset the stage timer |

---

## 3. Slide Cues & Automation Macros

### Slide Cues
Each slide can contain one or more automated cues:
- Triggered automatically when that slide goes live.
- Supports optional delay in milliseconds (`delay_ms`).
- Actions include: `NextSlide`, `PrevSlide`, `GotoSlide`, `BlackScreen`, `ClearOutput`, `TriggerProp`, `StartTimer`, `StopTimer`, `ResetTimer`.

### Macros
- Chains of trigger actions scheduled with millisecond intervals.
- Supports optional repeating / looping for automated pre-service countdowns or announcements.
- Managed from the Triggers sidebar tab or automated scripts.
