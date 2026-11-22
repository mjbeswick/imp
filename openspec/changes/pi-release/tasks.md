# Tasks

## 1. Measure

- [ ] 1.1 `scripts/measure.sh [host]` prints size, cold start and idle RSS
- [ ] 1.2 README shows M1 and Pi Zero 2 W figures

## 2. Cross-build

- [ ] 2.1 Sysroot with the device's libcurl and glibc under `target/sysroot/`
- [ ] 2.2 `deploy.sh` cross-builds and copies; `--native` keeps the old path
- [ ] 2.3 The deployed binary runs `--version` on the device

## 3. Device

- [ ] 3.1 dtach installed on the Hackberry; README example checked there
- [ ] 3.2 Keys card walked on the Hackberry keyboard; untypable keys listed
