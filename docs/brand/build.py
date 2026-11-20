#!/usr/bin/env python3
"""Renders the brand artwork in src/ to PNG with headless Chrome or Chromium.

    python3 docs/brand/build.py

Writes the app icons the bundles use (apps/super-keyboard/assets/icons/), the
images the README shows (docs/images/: banners, and the hero and Vietnamese
pictures composed from the real screenshots in shots/), and the brand kit
(docs/brand/png/).
Needs only Python and Chrome or Chromium; set $CHROME to its path if it is not
on PATH, and $CHROME_FLAGS for extra flags (for example --no-sandbox where the
system blocks Chrome's sandbox). Text renders with the fonts installed: Ubuntu
Sans and Fira Code, as docs/BRAND.md says.
"""

import os
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ICONS = ROOT / "apps" / "super-keyboard" / "assets" / "icons"
IMAGES = ROOT / "docs" / "images"
KIT = HERE / "png"

# HTML pages in composites/ built from the screenshots in shots/:
# (page, destination), rendered at 1600 x 900.
COMPOSITES = [
    ("hero.html", IMAGES / "hero.png"),
    ("vietnamese.html", IMAGES / "vietnamese.png"),
]

# (source in src/, width, height, destination)
TARGETS = [
    *[("logo-small.svg", s, s, ICONS / f"{s}x{s}.png") for s in (16, 24, 32)],
    *[("logo.svg", s, s, ICONS / f"{s}x{s}.png") for s in (48, 64, 128, 256, 512)],
    ("logo.svg", 1024, 1024, KIT / "logo-1024.png"),
    ("logo.svg", 512, 512, KIT / "logo-512.png"),
    ("lockup-dark.svg", 2080, 480, KIT / "lockup-dark.png"),
    ("lockup-light.svg", 2080, 480, KIT / "lockup-light.png"),
    ("social-dark.svg", 1280, 640, KIT / "social-dark.png"),
    ("social-light.svg", 1280, 640, KIT / "social-light.png"),
    ("banner-dark.svg", 1600, 560, IMAGES / "banner-dark.png"),
    ("banner-light.svg", 1600, 560, IMAGES / "banner-light.png"),
]


def find_chrome():
    if os.environ.get("CHROME"):
        return os.environ["CHROME"]
    for name in ("chromium", "chromium-browser", "google-chrome", "google-chrome-stable", "chrome"):
        if path := shutil.which(name):
            return path
    sys.exit("build.py: no Chrome or Chromium found; set $CHROME to its path")


def render(chrome, source, width, height, out, url=None):
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as profile:
        subprocess.run(
            [
                chrome, "--headless", "--disable-gpu", "--hide-scrollbars",
                f"--user-data-dir={profile}", "--default-background-color=00000000",
                f"--window-size={width},{height}", f"--screenshot={out}",
                *shlex.split(os.environ.get("CHROME_FLAGS", "")),
                url or f"{(HERE / 'frame.html').as_uri()}#{source}",
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    print(f"{out.relative_to(ROOT)}  {width}x{height}")


def main():
    chrome = find_chrome()
    for source, width, height, out in TARGETS:
        render(chrome, source, width, height, out)
    for page, out in COMPOSITES:
        render(chrome, page, 1600, 900, out, url=(HERE / "composites" / page).as_uri())
    # The scalable icon is the mark itself.
    shutil.copyfile(HERE / "src" / "logo.svg", ICONS / "super-keyboard.svg")
    print(f"{(ICONS / 'super-keyboard.svg').relative_to(ROOT)}  scalable")


if __name__ == "__main__":
    main()
