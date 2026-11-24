# Enemy Target Lock

A client-side native Rust mod for Horizon. It gently corrects aim toward hostile vanilla mobs
or other players only while the crosshair is already on them.

## Development

This is a Horizon-native mod project, not a Fabric/Forge mod. The launcher compiles it against
the exact Horizon build pinned in `horizon-mod.toml`.

```powershell
# Install Horizon's managed Rust toolchain once.
npm run setup

# Build for the installed "Mod Tester" instance.
npm run build

# Explicitly trust native code on the instance if it has not been trusted yet.
npm run build:trusted

# Check formatting
npm run fmt
```

The project intentionally stays one Cargo package: Horizon builds the mod from this root.
Keep the `Hooks` implementation in `src/lib.rs` (Horizon requires it at the crate root);
put feature logic in focused modules under `src/` as it is added. The launcher manages
the matching game SDK and mod tools during the first build; generated build output is ignored by Git.

Native mods run with the player's permissions and outside a sandbox. Only build/run this mod
if you trust its source.
