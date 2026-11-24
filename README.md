# Enemy Target Lock

A Native Rust mod for [Horizon](https://github.com/horizon-rust/horizon): lock onto a mob, inspired by soulslike games.

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

The listing describes the complete behavior: it can **target mobs and players**; **automatically adjust real
aim and projectile aim** while locked or nudging aim while not locked; and **redirect left/right clicks** to
the locked target. The owner of an integrated world can always use the mod. **Allow LAN Friends** controls
whether guests can use it in that world and is off by default; dedicated-server `allow_lock_on` is also off
by default.

The first control in **Options → Mods → Enemy Target Lock → Config** is the
**Enemy Target Lock** master toggle, above **Target Players**.

### Where lock-on works

| Where you play | Lock-on (toggleable) | Who decides | Default |
|---|---|---|---|
| Your integrated world | ✅ / ❌ | the owner is always allowed | on for owner |
| A friend's LAN world, **host has the mod** | ✅ / ❌ | the host's **Allow LAN Friends** controls guests | off for guests |
| A dedicated server, **server has the mod** | ✅ / ❌ | the server owner: `allow_lock_on` in `config/enemy_target_lock-server.toml` | off |
| Any server or LAN host **without a compatible mod handshake** (public servers, Realms, vanilla) | ❌ off | — | this mod disconnects its client |

Guests require the host to install the mod and explicitly opt in. The owner is always allowed in their
integrated world. A server without the mod never allows guests; if the optional permission channel is
unsupported, this mod disconnects its client. Players without this mod are unaffected.

### How the host's permission is checked (consent handshake)

| Step | Who | What happens |
|---|---|---|
| 1 | Your game | Joining someone else's world, asks the host once: "is lock-on allowed here?" |
| 2 | Host's game, **with the mod** | Answers with its switch: **Allow LAN Friends** in a game window that hosts LAN, `allow_lock_on` on a dedicated server. Remembers who asked. |
| 3a | Your game, answer **yes** | Lock-on works in this world. |
| 3b | Your game, answer **no** | You're disconnected with the reason: the host doesn't allow the mod in their world. |
| 3c | Host **without the mod** | The negotiated payload channel is unsupported, so your game disconnects immediately with an explanation. |
| 4 | Host changes its switch | Its game sends the new answer once to everyone who asked (a "no" disconnects them). |
| 5 | Your game leaves the world | The answer is forgotten; the next world is asked anew. |

The host sends periodic permission heartbeats so a lost revocation/update cannot leave a stale
grant active indefinitely.

### Behaviour when lock-on is off

| | |
|---|---|
| Middle-click | Does what it normally does (pick block). |
| Aim assist, projectile aim, clicks to target | Off. Clicks go where the crosshair is, as in vanilla. |
| Chat | One line per world explains why lock-on is off. |
| Joining | The server does not require this mod and unmodded players join normally. If you have this mod, it disconnects you when the server cannot negotiate its optional permission channel. |

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
