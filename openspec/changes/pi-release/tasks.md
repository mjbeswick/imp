# Tasks

## 1. Measure

- [x] 1.1 `scripts/measure.sh [host]` prints size, cold start and idle RSS
- [x] 1.2 README shows M1 and Pi Zero 2 W figures

## 2. Cross-build

- [x] 2.1 Sysroot with the device's libcurl and glibc under `target/sysroot/`
- [x] 2.2 `deploy.sh` cross-builds with `cargo zigbuild`, pinned to the device's glibc, and copies; `--native` keeps the old path
- [x] 2.3 The deployed binary runs `--version` on the device

## 3. Device

- [ ] 3.1 Keys card walked on the Hackberry keyboard; untypable keys listed
