# Enemy Target Lock

A Native Rust mod for [Horizon](https://github.com/horizon-rust/horizon): lock onto a mob and
fight without wrestling the camera.

## Features

| Feature | What it does |
|---|---|
| **Lock-on** | **Middle-click** locks onto the mob or player nearest the center of your view (within 24 blocks, in line of sight). Middle-click again to release. |
| **Switch target** | **Scroll** while locked: down for the next target on the right, up for the left. |
| **Smooth camera** | Your aim turns to the target quickly; the view glides after it. |
| **Clicks go to the target** | While locked, left / right clicks act on the locked target, without snapping the camera. |
| **Projectile aim** | Bows, crossbows, tridents and thrown items aim for drop and target movement while locked. |
| **Cinematic third person** | Lock in third person (F5) for an over-the-shoulder camera with a slightly narrower FOV. |
| **Auto-switch on kill** | When the target dies, the lock moves to the nearest other mob. |
| **Lock dot** | A small round dot marks the target; the crosshair is hidden while locked. |
| **Aim assist** | When not locked, a subtle nudge toward the mob under the crosshair. |

The lock ends when the target is gone, more than 32 blocks away, or out of sight for 3 seconds.
Everything is adjustable in-game (**Options > Mods > Enemy Target Lock > Config**) or in the commented
file `config/enemy_target_lock-client.toml`; the two stay in sync.

## Multiplayer and fair play

Lock-on, aim assist, clicks going to the target and projectile aim count as **aim assistance**, which many
servers forbid. So they only run where **the world's host has this mod and allows it**. Anywhere else they
stay off.

### Where lock-on works

| Where you play | Lock-on (toggleble) | Who decides | Default |
|---|---|---|---|
| Singleplayer (your own world) | ✅ | you | always on |
| Your world opened to LAN, friends present | ✅ / ❌ | you, the host: **Allow LAN Friends** (in-game Config); off also turns it off for you while friends are there | on |
| A friend's LAN world, **host has the mod** | ✅ / ❌ | the host's **Allow LAN Friends** | on |
| A dedicated server, **server has the mod** | ✅ / ❌ | the server owner: `allow_lock_on` in `config/enemy_target_lock-server.toml` | on |
| Any server or LAN host **without the mod** (public servers, Realms, vanilla) | ❌ off | — | always off |

Permission comes from the host installing the mod. A server without the mod never allows it, and each host
can turn it off with one switch.

### How the host's permission is checked (consent handshake)

| Step | Who | What happens |
|---|---|---|
| 1 | Your game | Joining someone else's world, asks the host once: "is lock-on allowed here?" |
| 2 | Host's game, **with the mod** | Answers with its switch: **Allow LAN Friends** in a game window that hosts LAN, `allow_lock_on` on a dedicated server. Remembers who asked. |
| 3a | Your game, answer **yes** | Lock-on works in this world. |
| 3b | Your game, answer **no** | You're disconnected with the reason: the host doesn't allow the mod in their world. |
| 3c | Host **without the mod** | Never answers. Your game asks again after 1, 2, 4 and 8 seconds, then stops. Lock-on stays off; you stay connected. |
| 4 | Host changes its switch | Its game sends the new answer once to everyone who asked (a "no" disconnects them). |
| 5 | Your game leaves the world | The answer is forgotten; the next world is asked anew. |

There's no polling: after the answer, nothing is sent unless the host changes its switch.

### Behaviour when lock-on is off

| | |
|---|---|
| Middle-click | Does what it normally does (pick block). |
| Aim assist, projectile aim, clicks to target | Off. Clicks go where the crosshair is, as in vanilla. |
| Chat | One line per world explains why lock-on is off. |
| Joining | Never blocked: the mod is `side = "client"`, so it isn't part of the server's mod check, and its messages are optional. |

### Network use

| Channel | Direction | Body | When |
|---|---|---|---|
| `enemy_target_lock:ask_lock_on` | your game → host | a protocol number | once on joining someone else's world (up to 4 retries while unanswered) |
| `enemy_target_lock:lock_on_answer` | host → your game | `allowed: true/false` | as the reply, and once whenever the host changes its switch |

Both are optional payloads, sent only to the world you're connected to. The mod makes no other network
connections.

### What it does not do

| | |
|---|---|
| Reach or hitbox changes | None. Clicks go to the target through the game's own targeting override, and the game still checks reach and line of sight; the server's own checks are unchanged. |
| Hiding from servers or anticheat | None. Nothing hides the mod, spoofs a mod list or works around a server's choice. |
| Automation | No auto-attack, auto-click or triggerbot: every attack is the player's own click. |
| Files | Only its own settings (Horizon's settings store) and its config files in `config/`. |
| Minecraft code or assets | None included. `src/ballistics.rs` holds a few vanilla projectile constants (launch speed, gravity, drag per item) used to aim; no game data tables. |

### Where this is in the code

| File | What |
|---|---|
| `src/session.rs` | Decides where targeting may run (the table above). |
| `src/lan.rs` | The consent handshake (ask, answer, push on change, disconnect on "no"). |
| `src/server_config.rs` | A dedicated server's `allow_lock_on` switch. |
| `src/attack.rs` | Clicks to the target, through the game's targeting override. |

See [docs/SPEC.md](docs/SPEC.md) for the full design.

## Development

This is a Horizon-native mod project, not a Fabric/Forge mod. The launcher compiles it against
the exact Horizon build pinned in `horizon-mod.toml`.
- Replace "My Instance" with your instance
- `horizon-launcher instance list` to show available instances
```
horizon-launcher native-build . --instance "My Instance" --trust-native-code
```
- Start the game after build
```
horizon-launcher play
```

The project intentionally stays one Cargo package: Horizon builds the mod from this root.
Keep the `Hooks` implementation in `src/lib.rs` (Horizon requires it at the crate root);
put feature logic in focused modules under `src/` as it is added. The launcher manages
the matching game SDK and mod tools during the first build; generated build output is ignored by Git.

Native mods run with the player's permissions and outside a sandbox. Only build/run this mod
if you trust its source.
