## External Dependencies
Some C code (sqlite) is included and built as part of the build (using the `cc` crate),
so a target C compiler needs to be configured.

## Packages
* binary tarballs (cirrus + restic)
* container images

All commits to main are built and packaged, using the version number from the latest changelog entry
and a build timestamp. An initial block headlined UNRELEASED is ignored, for collecting
changes incrementally. Version numbers just mark convenient points to easily identify roughly
which features a build has.
