# AppImage runtime notices

The pinned type-2 runtime statically links these components. Upstream license
texts are reproduced here, with their corresponding source projects:

- AppImage-MIT.txt: https://raw.githubusercontent.com/AppImage/type2-runtime/main/LICENSE
- musl-COPYRIGHT.txt: https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT
- libfuse-LGPL2.txt: https://raw.githubusercontent.com/libfuse/libfuse/master/LGPL2.txt
- squashfuse-LICENSE.txt: https://raw.githubusercontent.com/vasi/squashfuse/master/LICENSE
- zstd-LICENSE.txt: https://raw.githubusercontent.com/facebook/zstd/dev/LICENSE
- zlib-LICENSE.txt: https://raw.githubusercontent.com/madler/zlib/develop/LICENSE

Runtime source/build instructions: https://github.com/AppImage/type2-runtime

The native Studio executable and AppDir can also run independently of this runtime.
To replace/relink the runtime, build the upstream runtime and combine it with the
extracted SquashFS payload, or use appimagetool --runtime-file with your own build.

Pinned runtime reports upstream commit `fedfad5`; its exact binary SHA-256 is in
`packaging/studio/runtime.sha256`. The runtime is an independent executable; the
AppDir payload and native Studio binary can be used with a rebuilt runtime.
