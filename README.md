# Enemy Target Lock

A client-side native Rust mod for Horizon.

- **Middle-click** to lock onto the mob or player nearest the center of your view
  (within 24 blocks, in line of sight). You don't need to aim at it first.
- **Scroll** while locked to switch to the next target right or left.
- **Middle-click** again to release. The lock also ends when the target is gone, more than
  32 blocks away, or out of sight for 3 seconds.
- When the locked target dies, the lock moves on to the nearest other mob by itself.
- While locked, **left / right clicks** go to the target without snapping the camera.
- Bows, crossbows, tridents and thrown items aim for drop and target movement while locked.
- Lock in **third person** (F5) for a cinematic over-the-shoulder camera: the player turns to the
  target while the camera drifts smoothly behind, with a slightly narrower FOV.
- The locked target is marked with a round, see-through dot (size, opacity, colour and outline
  are adjustable), and the crosshair is hidden while locked.
- When not locked, a subtle aim assist gently corrects aim toward the mob under the crosshair.

Everything is adjustable in-game (**Options > Mods > Enemy Target Lock > Config**) or in the
commented file `config/enemy_target_lock-client.toml`. The two stay in sync, and changes apply
while the game runs.

See [docs/SPEC.md](docs/SPEC.md) for the full plan.

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
