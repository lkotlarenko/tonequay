# Releases and repository setup

Repository: https://github.com/lkotlarenko/tonequay

Before the first release, enable Actions and private vulnerability reporting,
review repository access and require passing CI for pull requests to the default
branch. Set the repository description to the first paragraph of README. Protect
version tags from accidental replacement. No signing key or provenance claim is
configured yet.

The application version is in `crates/tonequay/Cargo.toml`; engine crates retain
upstream versions. Update release notes and test reports, run `make test`, build
and smoke-test the AppImage outside the source tree on Wayland and X11. Check
both audio and MIDI on real hardware where available. Review all packaged files
and license notices. Never attach firmware packages to a release.

The GitHub release workflow builds a tagged version and uploads an AppImage,
native executable, license archive and checksums to a **draft release**. A maintainer reviews the
artifacts and publishes it. This automation is prepared locally and is not
verified until it has run in the new repository. Do not tag a release before the
packaging-tool setup and CI are working on the runner.

Local commands after adding the new repository remote (the application uses `main`; the existing upstream `master` is preserved):

```sh
git remote add tonequay https://github.com/lkotlarenko/tonequay.git
git push -u tonequay HEAD:main
# After CI and artifact validation:
git tag v0.1.0
git push tonequay v0.1.0
```

The original origin remains the upstream emulator until explicitly changed.
Inherited upstream release/Pages/catalog workflows were archived under
`docs/upstream/workflows/` so this fork does not accidentally publish upstream
plugins, browser builds or mirrored firmware. Their original sources remain
available for reference.
