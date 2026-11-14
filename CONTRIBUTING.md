# Contributing

Start with an issue describing the behavior, affected firmware and expected
result. Keep changes focused and preserve the existing emulator and CLI tests.
Do not commit firmware packages, personal saves, credentials or generated builds.
Inspect licenses before borrowing code or artwork; preserve attribution.

Run `make test` before opening a pull request. It checks Tonequay formatting and
strict Clippy, workspace Rust tests and browser regressions. Upstream crates
currently have pre-existing warnings/formatting differences, so application
linting is scoped deliberately. Add targeted tests for parsing, lifecycle and
routing changes; document limitations of hardware-dependent checks.

UI changes should include an app-only screenshot without private paths or
unlicensed artwork. Audio callbacks must remain bounded and free of allocation,
locks, logging, network and filesystem access. Do not alter the emulated clock
or skip CPU work to improve performance numbers. Include repeatable measurements
when proposing performance changes.

Treat other contributors respectfully. Discuss the code and evidence, avoid
personal attacks, and accept maintainer decisions about project scope.
