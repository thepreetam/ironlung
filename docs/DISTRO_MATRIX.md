# IronLung Target Distribution Matrix

Campaign C certifies IronLung across a matrix of Linux distributions covering ~95% of real-world deployments.

## Target Matrix

| Category | Distribution | Image | Glibc / Notes |
|----------|--------------|-------|---------------|
| Legacy enterprise | CentOS 7 | `centos:7` | glibc 2.17, kernel 3.10 |
| Legacy LTS | Ubuntu 18.04 | `ubuntu:18.04` | glibc 2.27 |
| Modern stable | Ubuntu 20.04 LTS | `ubuntu:20.04` | glibc 2.31 |
| Modern stable | Ubuntu 22.04 LTS | `ubuntu:22.04` | glibc 2.35 |
| Modern stable | Debian 11 (Bullseye) | `debian:11` | glibc 2.31 |
| Modern stable | Debian 12 (Bookworm) | `debian:12` | glibc 2.36 |
| Enterprise | Rocky Linux 9 | `rockylinux:9` | glibc 2.34 |
| Enterprise | RHEL 9 | `redhat/ubi9` | glibc 2.34 |
| Bleeding edge | Fedora 40 | `fedora:40` | glibc 2.38 |
| Bleeding edge | Arch Linux | `archlinux:latest` | Rolling |
| Minimal / musl | Alpine Linux | `alpine:3.19` | musl 1.2 (non-glibc) |

## Rationale

- **Legacy enterprise (CentOS 7)**: Still common in enterprise; glibc 2.17 and kernel 3.10 test oldest supported baseline.
- **Ubuntu 18.04–22.04**: Covers LTS releases; most cloud and desktop deployments.
- **Debian 11/12**: Stable server base; Debian’s conservative packaging tests compatibility.
- **Rocky/RHEL 9**: Enterprise Red Hat–family; UBI images for containerized builds.
- **Fedora 40 / Arch**: Newer toolchains and libraries; catches ABI or build issues early.
- **Alpine**: musl libc; non-glibc path; smaller footprint.

## Per-Distro Notes

### CentOS 7

- `yum` for packages; `libseccomp-devel` available.
- May need `centos-release-scl` + `devtoolset-9` for a newer gcc if Rust stable demands it.
- EOL June 2024; included for legacy compatibility.

### Ubuntu 18.04–22.04

- Standard `apt`; `build-essential`, `libseccomp-dev` provided.
- No special setup required.

### Debian 11/12

- Same deps as Ubuntu: `build-essential`, `libseccomp-dev`.

### Rocky Linux 9 / RHEL 9 (ubi9)

- `dnf`; packages: `gcc`, `libseccomp-devel`.
- UBI9 is minimal; add `tar`, `gzip`, `ca-certificates` for Rustup.

### Fedora 40

- `dnf`; packages: `gcc`, `libseccomp-devel`.

### Arch Linux

- `pacman -S base-devel libseccomp`.
- Rolling; image tag `latest` can change.

### Alpine 3.19

- `apk add build-base libseccomp-dev`.
- Uses musl libc; compatibility issues should be documented.
- IronLung targets glibc; musl is best-effort.

## Omitted Distributions

- **Ubuntu 16.04**: EOL; glibc 2.23; omitted to reduce CI complexity.
- **OpenSUSE**: Can be added later if needed.

## CI Integration

See `.github/workflows/distro-matrix.yml` for container-based runs. Each distro uses `scripts/ci-install-deps.sh` for package installation.

**Excluded from CI:** CentOS 7 and Ubuntu 18.04 are omitted from the workflow because GitHub’s Actions runner (Node 20) needs glibc ≥ 2.28 for post-job cleanup; these images provide glibc 2.17 and 2.27 respectively. They remain in the docs for local/manual testing.
