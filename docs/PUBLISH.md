# Publishing to the Horizon mod registry

How to get Enemy Target Lock listed on [playhorizon.dev/mods](https://playhorizon.dev/mods) and in the
launcher, through [horizon-rust/horizon-mods](https://github.com/horizon-rust/horizon-mods).

In short: attach a `.hmod` package to a GitHub release of this repo, then open a pull request to
`horizon-mods` that adds an entry pointing at it. A reviewer reads the source and merges it, and the mod
appears on the site and in the launcher within minutes.

Sources: the registry's [README](https://github.com/horizon-rust/horizon-mods/blob/main/README.md),
[MOD-POLICY.md](https://github.com/horizon-rust/horizon-mods/blob/main/MOD-POLICY.md) and
[REVIEWING.md](https://github.com/horizon-rust/horizon-mods/blob/main/REVIEWING.md), published native-mod
entries, and Horizon's modding book. Re-check the registry README before submitting: it says the registry is
partway through moving to a new package format, so the steps may change.

## Fix first, or the submission will be held

### 1. Targeting only where the world's owner allowed it (done)

The mod policy's cheat rules list **"aimbot or aim assist"** among the features that must "stay off on any
server that has not allowed them (or stay singleplayer-only)". The lock-on aiming, the aim assist and the
projectile aiming all fall under that. Horizon has no standard server opt-in yet (the policy says reviewers
don't hold a mod over how its opt-in is built), so the mod has its own consent handshake (`session.rs`,
`lan.rs`):

| Where | Lock-on |
|---|---|
| Your integrated world (private or opened to LAN) | off unless **Allow LAN Friends** is explicitly enabled; the API does not expose authoritative private/LAN status |
| A friend's LAN world | on only if their copy of the mod answers "allowed" (their **Allow LAN Friends**) |
| A dedicated server with the mod | off unless `allow_lock_on` is `true` in `config/enemy_target_lock-server.toml`; the default is off |
| Any other server, Realms, a host without the mod | off |

**Allow LAN Friends** is off by default and must be enabled as explicit permission for the integrated world and LAN guests. The API does not expose whether an integrated world is private or open to LAN, so it cannot infer private solo play from the tab list. A friend's game asks the host on joining
(retrying only while there's no answer: after 1, 2, 4 and 8 seconds, then never). A host that says
no disconnects the friend with the reason; a host that changes the setting sends the update to everyone who asked. Periodic heartbeats expire permission if updates are lost. No
answer (a host without the mod, any server) leaves lock-on off. Both messages are optional payloads and the mod
stays `side = "client"`, so it never blocks joining a server that doesn't have it. Mention this in the entry's description and the pull
request, so the reviewer finds it quickly.

### 2. Declare the licence (done)

`horizon-mod.toml` declares `license = "MIT"`, matching `LICENSE`, with `license_url`, `homepage` and `issues`
(the in-game Mods page's licence link and Homepage and Issues buttons). The registry entry's `license` must
match the repo's; an all-rights-reserved licence would need a special written grant.

## Prep

- **Put the code on `main`.** The entry's `source` link must point at the code the package was built from.
- **Optional, but it makes the page much better:**
  - `icon.png` in the repo root: a square PNG, at most 512×512 pixels and 256 KiB. It needs no manifest key.
  - A few screenshots.
  - A multiplayer warning in the description.

## Automatic build and draft release

`.github/workflows/build.yml` does steps 1 to 3 below on GitHub. It runs **only on a push to the `build`
branch**, so ordinary pushes to `main` don't build:

```sh
git push origin main:build      # build what's on main
```

It runs the unit tests, `cargo horizon-mod check` and `cargo horizon-mod build`, then uploads
`enemy_target_lock-<version>.hmod` and its SHA-256 (also in a `.sha256` file) to the **draft release
`v<version>`** (`version` in `horizon-mod.toml`). Each build replaces the draft's files and moves it to the new
commit; the tag is made when you publish the release.

The tool is built from the Horizon commit the registry's reviewers use (their `.github/horizon-rev`), and
cached. The SDK and Cargo's files are cached too, so later runs are much faster than the first. Once
`v<version>` is published it's never changed: later builds fail at the upload step until you bump
`version` in `horizon-mod.toml`, which starts a new draft.

Check the draft, then **publish it** and go on with step 4.

## Publishing steps

1. **Install the registry's checker** at the exact Horizon version the registry uses:

   ```sh
   cargo install --locked --git https://github.com/horizon-rust/horizon \
     --rev "$(curl -s https://raw.githubusercontent.com/horizon-rust/horizon-mods/main/.github/horizon-rev)" cargo-horizon-mod
   ```

2. **Check and build the package** from the mod folder:

   ```sh
   cargo horizon-mod check
   cargo horizon-mod build      # writes target/horizon-mod/enemy_target_lock.hmod
   ```

   Rename it with the version, e.g. `enemy_target_lock-0.1.0.hmod`.

3. **Make a GitHub release** of this repo (tag e.g. `v0.1.0`) and attach the `.hmod`. Note its hash:

   ```sh
   sha256sum enemy_target_lock-0.1.0.hmod
   ```

4. **Fork `horizon-rust/horizon-mods`** and add `mods/enemy_target_lock.toml`, in the shape of the published
   native mods:

   ```toml
   id = "enemy_target_lock"
   kind = "native-mod"
   name = "Enemy Target Lock"
   summary = "Middle-click to lock onto the nearest mob, with a smooth camera, projectile aiming and a cinematic third-person view."
   description = """
   ...what it does, controls, settings...

   **Singleplayer only:** lock-on and aim assist are off on servers.

   Native source mod: install it on the launcher's Mods page, press Build there, and confirm the native-code warning.
   It runs with your account's permissions. No Minecraft code or assets are included. MIT licensed."""
   authors = ["GoodDay360"]
   owners = ["GoodDay360"]
   owner_ids = [59399625]
   license = "MIT"
   source = "https://github.com/GoodDay360/enemy-target-lock"
   categories = ["..."]
   tags = ["lock-on", "combat", "camera"]

   [[versions]]
   version = "0.1.0"
   published = "YYYY-MM-DD"
   game = "26.3"
   file = "enemy_target_lock-0.1.0.hmod"
   sha256 = "<the hash from step 3>"
   changelog = "First release. Pinned to Horizon f1bb66f and Horizon Mixins e59da90."
   ```

   - `owner_ids` is the owner's numeric GitHub id (`gh api users/GoodDay360 --jq .id`). Only the owners can add
     later versions.
   - With an icon, also add `icons/enemy_target_lock.png` to the registry and set
     `icon = "icons/enemy_target_lock.png"` in the entry.
   - Optional: a [minisign](https://jedisct1.github.io/minisign/) public key in the entry. Every later version
     must then be signed with it, so nobody else can publish an update in your name.

5. **Open the pull request.** In its description, link the release file directly:

   ```text
   https://github.com/GoodDay360/enemy-target-lock/releases/download/v0.1.0/enemy_target_lock-0.1.0.hmod
   ```

6. **Wait for the checks, then a reviewer.**
   - The `stage` workflow downloads the linked file and checks its name and SHA-256.
   - The checks validate the package structure and the entry. They never compile or run the code.
   - A reviewer reads the entry, the report and the source. A native mod gets a security review.
   - On merge, the file becomes a release and the mod appears on the site and in the launcher within minutes.

## Updates

1. **Bump `version`** in `horizon-mod.toml` (e.g. `0.1.0` → `0.1.1`). A published file never changes, so every
   update needs a new number.
2. **Push to `build`** (`git push origin main:build`). The workflow builds it into a new draft release
   `v0.1.1`. Later pushes to `build` keep refreshing that draft until you publish it.
3. **Publish the draft.**
4. **In your fork of `horizon-mods`** (synced with upstream), edit `mods/enemy_target_lock.toml` and add a
   `[[versions]]` record **above** the previous one, with the new `version`, `published` date, `file` and
   `sha256` (from the release), and a `changelog`.
5. **Open a pull request** linking the new release file. Every native-mod version is reviewed, even an owner's.

When Horizon itself updates, `cargo horizon-mod update` lists what breaks in this mod and moves the pin in
`horizon-mod.toml`; fix what it reports, bump `version` and push.

## Not yet confirmed

- **Build proof.** Reviewers need proof that a native mod builds against its exact SDK, but the docs don't say
  what form it takes. Paste the `cargo horizon-mod check` output and a successful
  `horizon-launcher native-build` log into the pull request description.
- **The legacy submission form** at playhorizon.dev/mods/submit can't take current packages. Use the pull
  request.
