scan-primary-mirror
===================

`scan-primary-mirror` is a tool to scan the primary mirror for a
`MirrorManager2 <https://github.com/fedora-infra/mirrormanager2>`_ setup.

This is (another) rewrite of a MirrorManager2 Python tool in Rust just like
the `mirrorlist-server` and the `generate-mirrorlist-cache` tool which can be
found at https://github.com/adrianreber/mirrorlist-server.

`scan-primary-mirror` requires a database set up by MirrorManager2. It will
scan the configured primary mirror to see which files have changed since the
last scan based on the timestamps of the files.

The MirrorMamager2 crawler will use the information collected by
`scan-primary-mirror` to decide if a mirror is up to date or not.

It addition to scanning timestamps `scan-primary-mirror` will also detect
*repositories*. A *repository* is a directory with a `repodata` directory.

If a *repository* is found `scan-primary-mirror` will retrieve different
hashsums of the `repomd.xml` file which is then used by the
`mirrorlist-server` to create *metalinks* for YUM/DNF clients.
