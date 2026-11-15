# Releases and repository setup

Repository: https://github.com/lkotlarenko/tonequay

Actions, the application default branch and issues are enabled. Review private
vulnerability reporting,
review repository access and require passing CI for pull requests to the default
branch. The repository description and project topics are set. Protect
version tags from accidental replacement. No signing key or provenance claim is
configured yet.

The application version is in `crates/tonequay/Cargo.toml`; engine crates retain
upstream versions. Update release notes and test reports, run `make test`, build
and smoke-test the AppImage outside the source tree on Wayland and X11. Check
both audio and MIDI on real hardware where available. Review all packaged files
and license notices. Never attach firmware packages to a release.

The **Linux AppImage** workflow publishes automatically after every successful
build, firmware smoke, checksum check and artifact upload:

- Push a `v<application-version>` tag to publish a normal versioned release.
  The tag must match `crates/tonequay/Cargo.toml` exactly.
- Run the workflow manually to publish a prerelease with a unique
  `preview-<run-id>-<attempt>-<commit>` tag. Previews do not replace the latest
  stable release, and ordinary CI pushes do not publish anything.
- Assets are the exact build-job artifacts: AppImage, native executable,
  standalone license archive and SHA256SUMS. Publication uploads into a temporary
  draft first, then makes it public only once all uploads succeed.
- Existing published versions are not overwritten. Bump the application version
  for a new stable release; manual reruns get a distinct preview attempt tag.
- Release write permission exists only in the publish job. The workflow uses
  GitHub's built-in token; no personal access token or signing secret is needed.

The initial v0.1.0 release uses the already verified artifacts from Actions run
43631631012 at commit 3397070. No local build is substituted. Future runs use the
automatic publishing job. Build-tool and Rust inputs are pinned; signing and
byte-for-byte cross-host reproducibility remain separate qualification work.

Local commands after adding the new repository remote (the application uses `main`; the existing upstream `master` is preserved):

```sh
git remote add tonequay https://github.com/lkotlarenko/tonequay.git
git push -u tonequay HEAD:main
# First bump the app version to 0.2.1 and validate:
git tag v0.2.1
git push tonequay v0.2.1
```

The original origin remains the upstream emulator until explicitly changed.
Inherited upstream release/Pages/catalog workflows were archived under
`docs/upstream/workflows/` so this fork does not accidentally publish upstream
plugins, browser builds or mirrored firmware. Their original sources remain
available for reference.

## Verified publishing run

[Run 43632835803](https://github.com/lkotlarenko/tonequay/actions/runs/43632835803)
built and publicly published
[preview-43632835803-1-cf1c135c](https://github.com/lkotlarenko/tonequay/releases/tag/preview-43632835803-1-cf1c135c)
without manual intervention. It includes all four assets, and v0.1.0 remains
latest stable. Both the build and privileged publication jobs passed. This
qualifies manual preview publishing; future version-tag runs use the same upload
and publish path with tag/version validation and latest-stable promotion.
