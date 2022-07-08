# xous-semver

A simple utility for creating and serializing semantic versions.

The utility is primarily used by the helper crates which create and sign binaries for
Betrusted/Precursor. This includes utilities that create the EC and the SoC images.
This need for cross-platform packing of versions is what drives this into a separate
crate that can be included by both build systems.

This crate is designed to be runnable on host OS, Xous, or EC. The `std` feature
must be turned off to run on the EC.
