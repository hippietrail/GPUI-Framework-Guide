# Packaging NumNum

This directory holds everything needed to build native packages for every
desktop OS. The build output lands in `dist/`, which is gitignored.

| Artifact | Platform | Built by | Where |
|----------|----------|----------|-------|
| `.deb` (x86_64, aarch64) | Debian, Ubuntu | `cargo-deb` | `release.yml` / local |
| `.rpm` (x86_64, aarch64) | Fedora, RHEL, openSUSE | `cargo-generate-rpm` | `release.yml` / local |
| `.tar.xz` (x86_64, aarch64) | Linux | `tar` | `release.yml` / local |
| `.pkg.tar.zst` (x86_64, aarch64) | Arch, Arch Linux ARM | `makepkg` + `aur/PKGBUILD` | `release.yml` |
| `.dmg` (universal) | macOS | `packaging/macos/build-app.sh` | `release.yml` / local |
| `.exe` installer + `.zip` | Windows | NSIS | `release.yml` |
| `.pkg` + `.tar.xz` | FreeBSD | `packaging/freebsd/build-pkg.sh` | `release.yml` / local |

Every package installs the same three files: the `numnum` binary, a
`numnum.desktop` entry, and the `numnum.svg` launcher icon. The icons drawn
inside the app are embedded in the binary, so the binary is relocatable.

Release assets are all named `NumNum-<version>-<os>-<arch>.<ext>`; the macOS
build is universal, so it omits the arch.

## Layout

```
packaging/
  numnum.desktop          shared desktop entry
  linux/build-deb-rpm.sh  builds .deb + .rpm + tarball
  macos/Info.plist        app bundle metadata
  macos/build-app.sh      builds the universal .app and .dmg
  freebsd/MANIFEST.ucl.in pkg manifest template
  freebsd/pkg-plist       pkg file list
  freebsd/build-pkg.sh    builds the .pkg and a tarball
  aur/PKGBUILD            Arch binary package
```

The `.deb`/`.rpm` file layout lives in `[package.metadata.deb]` and
`[package.metadata.generate-rpm]` in the root `Cargo.toml`.

## CI

`.github/workflows/release.yml` runs on every `v*` tag. It creates the GitHub
release, then builds each platform on its own runner: Linux x86_64 and
aarch64, macOS and Windows natively, Arch packages in a container, FreeBSD
inside a VM. Every artifact is attached to the release. The repo is public,
so Actions minutes are unlimited; every job finishes well under the 6h cap.

## Cutting a release

1. Bump `version` in the root `Cargo.toml` and `pkgver` in
   `packaging/aur/PKGBUILD`. Commit.
2. Tag and push: `git tag v0.2.1 && git push origin v0.2.1`.
3. `release.yml` builds and attaches every artifact to the release.
4. Optional: the release already ships prebuilt `.pkg.tar.zst` files. To also
   list NumNum on the AUR, run `updpkgsums` in `packaging/aur/`, regenerate
   `.SRCINFO`, and push to the `numnum-bin` AUR repository.

## Building locally

Each format must be built on its own OS - the GUI stack (GPUI, wgpu,
Wayland/Vulkan) is not practical to cross-compile.

```sh
# Linux: .deb and .rpm  ->  dist/out/
cargo install cargo-deb cargo-generate-rpm
sh packaging/linux/build-deb-rpm.sh

# macOS: universal .app and .dmg  ->  dist/out/
sh packaging/macos/build-app.sh

# FreeBSD: .pkg and tarball  ->  dist/out/
sh packaging/freebsd/build-pkg.sh
```

## FreeBSD notes

Modern `pkg(8)` produces a `.pkg` file (a tar archive, zstd-compressed by
default); the old `.txz` extension is gone. `build-pkg.sh` needs no ports
tree: it stages the files, derives runtime dependencies from the binary with
`ldd` + `pkg which`, generates a `+MANIFEST` from `MANIFEST.ucl.in`, and runs
`pkg create -r <stage> -m <meta> -p pkg-plist`. The CI VM targets FreeBSD
14.2; for a FreeBSD 15 package, run the script on a 15 host so the ABI matches.

## Signing

The macOS `.dmg` and the Windows `.zip` are produced **unsigned**. Public
distribution also needs:

- macOS: `codesign` with a Developer ID certificate and notarization via
  `xcrun notarytool`.
- Windows: Authenticode signing of the binary.

Add the certificates as repository secrets and wire them into `release.yml`
when they are available.

## cargo-dist

`[workspace.metadata.dist]` in the root `Cargo.toml` is left in place for
anyone who later wants cargo-dist's `curl | sh` installer, PowerShell
installer or Homebrew formula on top of these packages. It is not used by
`release.yml`. To adopt it, run `cargo install cargo-dist` then `dist init`,
and merge its generated workflow with this one.
