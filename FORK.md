# espaciofuturo fork of takumi

This is `espaciofuturoio/takumi`, our own build of [kane50613/takumi](https://github.com/kane50613/takumi).
We carry features we need now (CSS 3D transforms first) and follow upstream every day. We do not
wait on the maintainer: upstreaming a patch is optional and comes later. Nothing is posted on
`kane50613/takumi` from here. The model is the same as our Electric Agents fork
(`espaciofuturoio/electric`): upstream code plus a short, named list of patches, released as
tarballs that install by URL.

## Branches and tags

| Ref                              | What it is                                                | Who moves it                    |
| -------------------------------- | --------------------------------------------------------- | ------------------------------- |
| `master`                         | Exact mirror of upstream `master`. Never commit here.     | EF Sync (fast-forward only)     |
| `espaciofuturo` (default branch) | `master` + our patches, rebased. Our workflows live here. | EF Sync (rebase), people via PR |
| `v<upstream version>-ef.<n>`     | A tested release of `espaciofuturo`, e.g. `v2.14.0-ef.1`  | EF Release                      |

`<upstream version>` is `takumi-js/package.json`'s version on the commit released. Upstream
`master` runs ahead of its npm release, so `2.14.0-ef.1` means "upstream master after 2.14.0 plus
our patches", not "npm 2.14.0 plus our patches". `<n>` counts up per upstream version.

Upstream's own workflows (`ci.yml`, `mirror.yml`, `tegami-pr.yml`) are disabled on this fork in the
Actions settings; they would try to publish to npm and mirror to Codeberg.

## Patches

`git log --oneline master..espaciofuturo` is the source of truth. Today:

1. **3D transforms** (`takumi-core`, `takumi-raster`).
   - `perspective()`, `rotateX/Y/Z()`, `rotate3d()`, `translateZ()` and `matrix3d()` in `transform`.
   - The `perspective` and `perspective-origin` properties (applied to the children, as CSS does)
     and `backface-visibility: hidden`.
   - How: the local transform composes as a 4x4 and flattens to the `z = 0` plane. When the
     result is affine (e.g. `rotateX` without perspective) the normal 2D path paints it. When it
     is projective, the element paints flat into an effect layer bounded to its flat box plus its
     projected box, and the layer is resampled bilinearly through the device homography. Filters,
     clip-path and masks apply before the projection, opacity and blend after (as Chrome does).
   - Keyframe animations use the same path. `rotateX/Y/Z`, `translateZ`, `perspective()` and
     same-axis `rotate3d` interpolate; `matrix3d` interpolates discretely.
   - Fixtures: `style_transform_perspective`, `style_perspective_cube`, `style_transform_3d_card`,
     `animation_keyframe_rotate_y`. Headless Chrome on the same HTML differs on 0.11 %, 0.029 % and
     0.007 % of pixels (12 % fuzz); the residue is text antialiasing.
   - Bench `cargo bench -p takumi --bench transform_3d` (M2 Max, 1080x1920, one 600x400 card):
     tilted card 2.44 ms; the first prototype's full-viewport layer took 5.66 ms; a flat
     `rotate(4deg)` card takes 4.12 ms.
2. **Fork infrastructure**: this file, `scripts/ef-*`, `.github/workflows/ef-*.yml`.

### 3D limits (by design, not bugs)

- No `transform-style: preserve-3d`. Each element is flattened into its parent, as with
  `transform-style: flat`. A cube is built from sibling faces: give the parent `perspective`, give
  each face `rotateY(<angle>) translateZ(<half size>)` and `backface-visibility: hidden`. With
  hidden back faces the visible faces of a convex solid never overlap, so no depth sorting is
  needed (`style_perspective_cube` matches Chrome). A face may also carry its own
  `perspective()` as the first function instead of the parent property.
- Angles interpolate along the shorter arc (upstream `Angle` normalises to [0, 360)). A keyframe
  step must turn less than 180 degrees; split a longer spin into more keyframes.
- SVG and PDF output paint 3D-transformed elements flat. Raster (PNG/WebP/JPEG/video frames) is
  exact. `measure()` reports the flat box.
- Text is rasterised flat, then resampled, so it is slightly softer than Chrome's at steep angles.

## Release

`EF Release` (`.github/workflows/ef-release.yml`, on demand or called by EF Sync):

1. Runs `cargo test --locked` and fails if the run changes any committed golden.
2. Builds `@takumi-rs/core` for `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu` (napi-cross,
   glibc 2.31) and `x86_64-unknown-linux-musl` (zigbuild, tested in Alpine), plus
   `@takumi-rs/helpers`, `@takumi-rs/wasm` and `takumi-js`, with upstream's toolchain pins.
3. `scripts/ef-pack.ts` packs four tarballs with the `-ef.<n>` version. Their dependencies on each
   other point to the sibling tarballs' URLs. `@takumi-rs/core` carries all three `.node` binaries
   and no per-platform `optionalDependencies`; the napi loader finds `core.<target>.node` next to
   `dist` first.
4. Publishes the GitHub Release `v<version>` (prerelease) and installs it on darwin-arm64,
   linux-x64-gnu and Alpine (musl) with `scripts/ef-smoke/smoke.ts`, which fails unless the native
   core projects a 3D transform. A failed smoke renames the release "BROKEN".

Why tarballs on GitHub Releases and not GitHub Packages: the fork is public, so the release URLs
need no token in bun, Docker or CI. GitHub Packages' npm registry needs a token for every install,
even of a public package, which would add a secret to every Dockerfile and workflow that installs
the consumer's workspace. Names stay upstream's (`takumi-js`, `@takumi-rs/*`), so imports do not
change.

Consume (bun):

```json
"takumi-js": "https://github.com/espaciofuturoio/takumi/releases/download/v2.14.0-ef.1/takumi-js-2.14.0-ef.1.tgz"
```

### WASM fallback

`@takumi-rs/wasm` is built from the same commit and shares `takumi-raster`, so it renders 3D too.
`takumi-js` falls back to it only when you pass a WASM `module`/renderer; it does not switch by
itself when the native addon fails to load. It is a workable no-native fallback for a platform we
do not build (linux-arm64, Windows), not a production path for video: it is single-threaded and
slower than the addon.

## Sync

`EF Sync` (`.github/workflows/ef-sync.yml`) runs daily at 09:17 UTC and on demand:

1. `scripts/ef-sync.sh --push` fetches upstream `master`, fast-forwards our `master` to it and
   rebases `espaciofuturo` onto it (`--force-with-lease`). Nothing to do when upstream did not move.
2. It then calls EF Release on the rebased commit: tests, builds, publishes `-ef.<n+1>`.
3. On a rebase conflict it opens an issue on this fork (label `ef-sync`) listing the conflicting
   files; on a test/build failure it opens one with the run link. `espaciofuturo` may then sit
   rebased but unreleased; consumers only ever pin a release tag.

By hand (also the fix for a conflict):

```bash
git fetch upstream master && git checkout espaciofuturo
bash scripts/ef-sync.sh            # dry run: rebases locally, pushes nothing
# on conflict: git rebase upstream/master, resolve, cargo test --locked, then
git push origin upstream/master:master && git push --force-with-lease origin espaciofuturo
gh workflow run ef-release.yml -R espaciofuturoio/takumi -f ref=espaciofuturo
```

### Sync token

Pushing upstream commits that edit `.github/workflows/` needs a token with the `workflow` scope,
which `GITHUB_TOKEN` can never have (upstream edited its workflows on ~20 of 95 days, Jul-Sep 2026).
EF Sync uses the repository secret `EF_SYNC_TOKEN` when it exists (a fine-grained PAT on this
repository with Contents, Workflows and Issues: read/write), else `GITHUB_TOKEN`. Without the
secret those days end in an issue and a one-command manual sync.

## Consumers

- `espaciofuturoio/neo-real-estate`: `apps/creative-renderer` pins a release tarball (see its
  `official-docs/reference/supersessions.md`, key `neo.deps.takumi-upstream-npm`). Bump the URL to
  take a new release; nothing else changes.
