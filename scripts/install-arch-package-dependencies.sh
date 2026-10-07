#!/usr/bin/env bash
# Arch container setup for packaging a prebuilt Synara binary. Installs the
# PKGBUILD runtime depends (so makepkg's dependency check and the ldd probe in
# build-arch-package.sh see the real Arch libraries) and repo-add tooling.
set -euo pipefail

pacman -Syu --noconfirm
pacman -S --needed --noconfirm \
  dbus \
  enchant \
  gtk3 \
  hunspell-en_us \
  libsecret \
  webkit2gtk-4.1 \
  xdg-desktop-portal \
  gst-plugins-good \
  libayatana-appindicator \
  librsvg \
  openssl \
  file \
  pacman-contrib
