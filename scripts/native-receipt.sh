#!/usr/bin/env bash
# Local native-build proof for the latest draft release:
#   1. finds the newest draft release and fails if it already has a log or receipt,
#   2. downloads its .hmod, checks it against its .sha256 and extracts it unchanged,
#   3. native-builds that exact package with horizon-launcher, capturing the log,
#   4. starts the game on the build and asks you whether it worked,
#   5. writes the receipt and uploads it with the log to the same draft.
# Anything wrong stops the script with the reason, and nothing gets uploaded.
#
# Usage: scripts/native-receipt.sh
# Optional env:
#   INSTANCE=Horizon   launcher instance to build with and play
#   REPO=owner/name    default: the GitHub repo of this checkout
#   SKIP_PLAY=1        don't start the game (no in-game check)
set -euo pipefail

LOG_NAME=native-launcher-build.log
RECEIPT_NAME=native-launcher-receipt.txt
WORKFLOW_NAME="Build mod"
INSTANCE=${INSTANCE:-Horizon}

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$*"; }

for tool in gh horizon-launcher sha256sum unzip; do
  command -v "$tool" > /dev/null || die "'$tool' is not installed or not on PATH."
done
gh auth status > /dev/null 2>&1 || die "gh is not signed in. Run: gh auth login"

REPO=${REPO:-$(gh repo view --json nameWithOwner --jq .nameWithOwner 2> /dev/null)} \
  || die "couldn't work out the GitHub repo. Set REPO=owner/name."

# --- 1. Latest draft, without the files yet -----------------------------------------
step "Finding the latest draft release in $REPO"
TAG=$(gh release list -R "$REPO" --limit 100 --json tagName,isDraft,createdAt \
  --jq '[.[] | select(.isDraft)] | sort_by(.createdAt) | last | .tagName // empty')
[ -n "$TAG" ] || die "no draft release in $REPO. Push to the build branch so the workflow makes one."
echo "Draft: $TAG"

ASSETS=$(gh release view "$TAG" -R "$REPO" --json assets --jq '.assets[].name')
for name in "$LOG_NAME" "$RECEIPT_NAME"; do
  if grep -qxF "$name" <<< "$ASSETS"; then
    die "draft $TAG already has '$name'. Nothing was built or uploaded.
  If it's out of date, remove it first: gh release delete-asset $TAG $name -R $REPO"
  fi
done

HMODS=$(grep '\.hmod$' <<< "$ASSETS" || true)
[ -n "$HMODS" ] || die "draft $TAG has no .hmod asset."
[ "$(wc -l <<< "$HMODS")" = 1 ] || die "draft $TAG has more than one .hmod asset: $(tr '\n' ' ' <<< "$HMODS")"
HMOD=$HMODS
grep -qxF "$HMOD.sha256" <<< "$ASSETS" || die "draft $TAG has $HMOD but no $HMOD.sha256."

COMMIT=$(gh release view "$TAG" -R "$REPO" --json targetCommitish --jq .targetCommitish)
[[ $COMMIT =~ ^[0-9a-f]{40}$ ]] || die "draft $TAG targets '$COMMIT', not a commit SHA."
RUN_URL=$(gh run list -R "$REPO" -w "$WORKFLOW_NAME" --commit "$COMMIT" --status success \
  --limit 1 --json url --jq '.[0].url // empty')
[ -n "$RUN_URL" ] || die "no successful '$WORKFLOW_NAME' run for commit $COMMIT."

# --- 2. The exact staged package ----------------------------------------------------
WORK=$(mktemp -d "${TMPDIR:-/tmp}/native-receipt.XXXXXX")
step "Downloading $HMOD into $WORK"
gh release download "$TAG" -R "$REPO" -p "$HMOD" -p "$HMOD.sha256" -D "$WORK"
(cd "$WORK" && sha256sum -c --quiet "$HMOD.sha256") \
  || die "$HMOD on draft $TAG doesn't match its .sha256."
SHA=$(sha256sum "$WORK/$HMOD" | cut -d' ' -f1)
echo "SHA-256: $SHA"

UNPACKED="$WORK/unpacked"
unzip -q "$WORK/$HMOD" -d "$UNPACKED" || die "couldn't extract $HMOD (is it a zip?)."
MANIFEST="$UNPACKED/horizon-mod.toml"
[ -f "$MANIFEST" ] || die "$HMOD has no horizon-mod.toml at its root."

native_pin() { sed -n "/^\[native\]/,/^\[/ s/^$1 = \"\(.*\)\"\$/\1/p" "$MANIFEST" | head -n1; }
GAME=$(native_pin game)
SDK_COMMIT=$(native_pin horizon)
MIXINS_COMMIT=$(native_pin mixins)
[ -n "$GAME" ] || die "[native] game is missing from the package's horizon-mod.toml."
[[ $SDK_COMMIT =~ ^[0-9a-f]{40}$ ]] || die "[native] horizon in the package isn't a full commit: '$SDK_COMMIT'."
[[ $MIXINS_COMMIT =~ ^[0-9a-f]{40}$ ]] || die "[native] mixins in the package isn't a full commit: '$MIXINS_COMMIT'."

# --- 3. Native build ----------------------------------------------------------------
LOG="$WORK/$LOG_NAME"
step "Native-building $HMOD for instance '$INSTANCE'"
CMD=(horizon-launcher native-build "$UNPACKED" --instance "$INSTANCE" --trust-native-code)
echo "${CMD[*]}"
set +e
"${CMD[@]}" 2>&1 | tee "$LOG"
STATUS=${PIPESTATUS[0]}
set -e
[ "$STATUS" = 0 ] || die "native build failed (exit $STATUS). Nothing was uploaded. Log: $LOG"

BUILD_ID=$(grep -oE 'native-[0-9a-f]{7,}-[0-9a-f]+' "$LOG" | tail -n1 || true)
[ -n "$BUILD_ID" ] || die "the build finished but its build id (native-...) isn't in the log. Nothing was uploaded. Log: $LOG"
[[ $BUILD_ID == native-"${SDK_COMMIT:0:7}"-* ]] \
  || die "build $BUILD_ID isn't on the pinned SDK ${SDK_COMMIT:0:7}. Nothing was uploaded."
LAUNCHER_VERSION=$(horizon-launcher --version | head -n1)

# --- 4. Start the game --------------------------------------------------------------
if [ "${SKIP_PLAY:-}" != 1 ]; then
  step "Starting the game on $BUILD_ID (instance '$INSTANCE')"
  horizon-launcher play --instance "$INSTANCE" || die "the game failed to start. Nothing was uploaded."
  echo
  read -r -p "Did the game load with the mod working? [y/N] " answer
  [[ $answer =~ ^[Yy]$ ]] || die "you said the in-game check failed. Nothing was uploaded. Check: horizon-launcher logs"
fi

# --- 5. Receipt and upload ----------------------------------------------------------
RECEIPT="$WORK/$RECEIPT_NAME"
cat > "$RECEIPT" <<EOF
Native launcher compile/link receipt

Package: $HMOD
Package SHA-256: $SHA
Package asset: https://github.com/$REPO/releases/download/$TAG/$HMOD
Package source commit: $COMMIT
Package workflow: $RUN_URL

Signed Horizon SDK release:
https://github.com/horizon-rust/horizon/releases/tag/sdk-$SDK_COMMIT
Horizon SDK commit: $SDK_COMMIT
Game: $GAME
Horizon Mixins commit: $MIXINS_COMMIT

Launcher: $LAUNCHER_VERSION
Command: ${CMD[*]}
Input: the exact staged .hmod above, extracted without modification
Result: successful native compile and link; build $BUILD_ID

Build output is attached as $LOG_NAME. This launcher run used a
locally prepared licensed game install; the hosted workflow does not run
native-build, and no account credentials were sent to GitHub.
EOF
step "Receipt"
cat "$RECEIPT"

# Check again: the draft may have changed while the build and game ran.
STATE=$(gh release view "$TAG" -R "$REPO" --json isDraft,assets \
  --jq 'if .isDraft then (.assets[].name) else "<published>" end')
grep -qxF '<published>' <<< "$STATE" && die "$TAG was published meanwhile. Nothing was uploaded."
for name in "$LOG_NAME" "$RECEIPT_NAME"; do
  grep -qxF "$name" <<< "$STATE" && die "'$name' was added to $TAG meanwhile. Nothing was uploaded."
done
grep -qxF "$HMOD" <<< "$STATE" || die "$HMOD is gone from $TAG. Nothing was uploaded."
NOW_SHA=$(gh release view "$TAG" -R "$REPO" --json assets \
  --jq ".assets[] | select(.name == \"$HMOD\") | .digest" | sed 's/^sha256://')
[ -z "$NOW_SHA" ] || [ "$NOW_SHA" = "$SHA" ] \
  || die "$HMOD on $TAG changed meanwhile (now $NOW_SHA). Nothing was uploaded; run again."

step "Uploading to draft $TAG"
# No --clobber: if either name exists by now, gh refuses instead of replacing it.
gh release upload "$TAG" -R "$REPO" "$LOG" "$RECEIPT" \
  || die "upload failed. Check the draft: gh release view $TAG -R $REPO"

echo "Done. Proof links once $TAG is published:"
echo "  https://github.com/$REPO/releases/download/$TAG/$RECEIPT_NAME"
echo "  https://github.com/$REPO/releases/download/$TAG/$LOG_NAME"
echo "Local copies: $WORK"
