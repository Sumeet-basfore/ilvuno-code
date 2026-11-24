#!/usr/bin/env bash
# Build the three Linux packages and publish them as a GitHub release.
#
#   tools/release.sh v0.0.1
#
# Runs on main only, from a clean tree, and refuses a tag that already exists, so a release always
# matches a commit that is on the remote. The files land in target/release/<tag>/ with a SHA256SUMS.
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "$*" >&2; exit 1; }

TAG="${1:-}"
[ -n "$TAG" ] || die "usage: tools/release.sh v0.0.1"
case "$TAG" in
  v[0-9]*) ;;
  *) die "the tag is v0.0.1, with the v" ;;
esac

[ "$(git rev-parse --abbrev-ref HEAD)" = main ] \
  || die "you are on $(git rev-parse --abbrev-ref HEAD). Merge the pull requests, then run this on main."
[ -z "$(git status --porcelain)" ] || die "the tree has uncommitted changes. Commit or stash them first."

command -v gh >/dev/null || die "gh is not installed"

git fetch -q origin main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || die "main is not pushed. Run: git push origin main"

git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "tag $TAG already exists"
gh release view "$TAG" >/dev/null 2>&1 && die "a release for $TAG already exists"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ "v$version" = "$TAG" ] || die "Cargo.toml says $version, so the tag should be v$version"

./tools/package-linux.sh

bundle=target/release/bundle
stage=target/release/$TAG
rm -rf "$stage"
mkdir -p "$stage"
find "$bundle" -maxdepth 2 -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) -exec cp {} "$stage/" \;

count=$(find "$stage" -maxdepth 1 -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) | wc -l)
[ "$count" -eq 3 ] || die "expected three packages, found $count"

(cd "$stage" && sha256sum *.deb *.rpm *.AppImage >SHA256SUMS && ls -lh)

notes=$(mktemp)
cat >"$notes" <<EOF
## Ilvuno $TAG

A desktop app where several coding agents work on one project, in rounds, until a goal is met.
You write the goal and say what finished looks like. The agents work in threads, write what they
found to a shared board, and nothing lands until you accept it at the gate.

Each package holds the window (\`/usr/bin/ilvuno-window\`), the command line (\`/usr/bin/ilvuno\`),
and the local runner (\`/usr/lib/Ilvuno/ilvunod\`).

| File | For | Install |
|---|---|---|
| \`Ilvuno_${version}_amd64.deb\` | Debian, Ubuntu | \`sudo apt install ./Ilvuno_${version}_amd64.deb\` |
| \`Ilvuno-$version-1.x86_64.rpm\` | Fedora, openSUSE | \`sudo dnf install ./Ilvuno-$version-1.x86_64.rpm\` |
| \`Ilvuno_${version}_amd64.AppImage\` | any Linux | \`chmod +x\` it, then run it |

The rpm is unsigned. \`SHA256SUMS\` holds the checksum of each file. You need one coding-agent
command already installed and signed in, such as \`claude\` or \`codex\`. Step by step, and where your
data goes: https://github.com/Sumeet-basfore/ilvuno-code/blob/main/docs/install.md
EOF

git tag -a "$TAG" -m "Ilvuno $TAG"
git push origin "$TAG"

# The first release is the whole app, so the notes above are the changelog. Later ones can add
# --generate-notes to append the commit list.
gh release create "$TAG" "$stage"/* \
  --verify-tag \
  --title "Ilvuno ${TAG#v}" \
  --notes-file "$notes"

rm -f "$notes"
gh release view "$TAG" --json url -q .url
