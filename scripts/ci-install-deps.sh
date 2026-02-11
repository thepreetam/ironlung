#!/bin/bash
# Install build dependencies for IronLung in CI containers.
# Usage: ./scripts/ci-install-deps.sh [distro]
# distro: centos7, ubuntu1804, ubuntu2004, ubuntu2204, debian11, debian12,
#         rockylinux9, rhel9, fedora40, arch, alpine

set -e

DISTRO="${1:-unknown}"

case "$DISTRO" in
    centos7)
        yum install -y gcc libseccomp-devel curl ca-certificates
        ;;
    ubuntu1804|ubuntu2004|ubuntu2204)
        apt-get update -qq && apt-get install -y -qq build-essential libseccomp-dev curl ca-certificates
        ;;
    debian11|debian12)
        apt-get update -qq && apt-get install -y -qq build-essential libseccomp-dev curl ca-certificates
        ;;
    rockylinux9|rhel9)
        dnf install -y --allowerasing gcc libseccomp-devel curl tar gzip ca-certificates
        ;;
    fedora40)
        dnf install -y gcc libseccomp-devel curl ca-certificates
        ;;
    arch)
        pacman -Sy --noconfirm base-devel libseccomp curl ca-certificates
        ;;
    alpine)
        apk add build-base libseccomp-dev curl ca-certificates
        ;;
    *)
        echo "Unknown distro: $DISTRO" >&2
        echo "Supported: centos7, ubuntu1804, ubuntu2004, ubuntu2204, debian11, debian12, rockylinux9, rhel9, fedora40, arch, alpine" >&2
        exit 1
        ;;
esac
