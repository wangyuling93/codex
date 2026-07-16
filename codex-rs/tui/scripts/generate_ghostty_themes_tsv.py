#!/usr/bin/env python3
"""Generate codex-rs/tui/src/render/ghostty_themes.tsv from Ghostty theme files.

Upstream source (iTerm2-Color-Schemes Ghostty export):
  https://github.com/mbadolato/iTerm2-Color-Schemes/tree/fc73ce39746540b6d5ec6b91e304785431401d85/ghostty

Pinned reference for the currently checked-in catalog:
  Ghostty 1.3.1 uses the iTerm2-Color-Schemes release tag
  release-20260216-151611-fc73ce3, whose commit is
  fc73ce39746540b6d5ec6b91e304785431401d85 (MIT, Copyright (c) 2011 to
  Present Mark Badolato). Individual palette copyrights belong to their
  authors; see the TSV header and the repo root NOTICE file.

Usage:
  # From a clean local checkout of iTerm2-Color-Schemes at the pinned commit:
  git -C /path/to/iTerm2-Color-Schemes checkout --detach \\
      fc73ce39746540b6d5ec6b91e304785431401d85
  python3 generate_ghostty_themes_tsv.py \\
      --source-dir /path/to/iTerm2-Color-Schemes/ghostty \\
      --output ../src/render/ghostty_themes.tsv

  # Validate the checked-in TSV without regenerating:
  python3 generate_ghostty_themes_tsv.py --verify ../src/render/ghostty_themes.tsv

Each Ghostty theme file looks like:

  palette = 0=#rrggbb
  ...
  palette = 15=#rrggbb
  background = #rrggbb
  foreground = #rrggbb

Names are normalized to kebab-case with a mandatory ``ghostty-`` prefix.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

PINNED_SOURCE_COMMIT = "fc73ce39746540b6d5ec6b91e304785431401d85"
PINNED_THEME_COUNT = 463
PINNED_SOURCE_URL = (
    "https://github.com/mbadolato/iTerm2-Color-Schemes/tree/"
    f"{PINNED_SOURCE_COMMIT}/ghostty"
)
LICENSE_HEADER = f"""\
# Generated from the themes bundled with Ghostty 1.3.1.
# Source: {PINNED_SOURCE_URL}
# Source revision: {PINNED_SOURCE_COMMIT}
# Columns: name, background, foreground, palette colors 0 through 15.
# Regenerate: codex-rs/tui/scripts/generate_ghostty_themes_tsv.py
#
# MIT License
# Copyright (c) 2011 to Present Mark Badolato
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.
#
# This license covers the iTerm2-Color-Schemes repository collection of themes.
# The copyright/license for each individual theme belongs to the author of that
# theme.
"""

PALETTE_RE = re.compile(
    r"^palette\s*=\s*(?P<index>\d+)\s*=\s*#?(?P<rgb>[0-9A-Fa-f]{6})\s*$"
)
BG_RE = re.compile(r"^background\s*=\s*#?(?P<rgb>[0-9A-Fa-f]{6})\s*$")
FG_RE = re.compile(r"^foreground\s*=\s*#?(?P<rgb>[0-9A-Fa-f]{6})\s*$")
DATA_RE = re.compile(
    r"^ghostty-[^\t]+\t[0-9A-Fa-f]{6}\t[0-9A-Fa-f]{6}"
    r"(?:\t[0-9A-Fa-f]{6}){16}$"
)


def to_kebab(stem: str) -> str:
    name = stem.strip().lower().replace("+", " plus ")
    name = re.sub(r"[^a-z0-9]+", "-", name)
    name = re.sub(r"-+", "-", name).strip("-")
    if not name:
        raise ValueError(f"empty theme name from stem {stem!r}")
    return f"ghostty-{name}"


def parse_theme_file(path: Path) -> tuple[str, str, str, list[str]]:
    background: str | None = None
    foreground: str | None = None
    palette: dict[int, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if m := PALETTE_RE.match(line):
            palette[int(m.group("index"))] = m.group("rgb").lower()
            continue
        if m := BG_RE.match(line):
            background = m.group("rgb").lower()
            continue
        if m := FG_RE.match(line):
            foreground = m.group("rgb").lower()
            continue
    if background is None:
        raise ValueError(f"{path}: missing background")
    if foreground is None:
        raise ValueError(f"{path}: missing foreground")
    colors = []
    for index in range(16):
        if index not in palette:
            raise ValueError(f"{path}: missing palette index {index}")
        colors.append(palette[index])
    return to_kebab(path.name), background, foreground, colors


def verify_source_checkout(source_dir: Path) -> None:
    try:
        head = subprocess.run(
            ["git", "-C", str(source_dir), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        status = subprocess.run(
            [
                "git",
                "-C",
                str(source_dir),
                "status",
                "--short",
                "--untracked-files=all",
                "--",
                ".",
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (FileNotFoundError, subprocess.CalledProcessError):
        raise SystemExit(
            f"{source_dir}: source must be a Git checkout at "
            f"{PINNED_SOURCE_COMMIT}"
        ) from None

    if head != PINNED_SOURCE_COMMIT:
        raise SystemExit(
            f"{source_dir}: expected source revision {PINNED_SOURCE_COMMIT}, got {head}"
        )
    if status:
        raise SystemExit(
            f"{source_dir}: source has local changes; regenerate from a clean checkout"
        )


def generate(source_dir: Path) -> str:
    verify_source_checkout(source_dir)
    rows: list[tuple[str, str, str, list[str]]] = []
    for path in sorted(source_dir.iterdir(), key=lambda p: p.name.lower()):
        if not path.is_file() or path.name.startswith("."):
            continue
        rows.append(parse_theme_file(path))
    rows.sort(key=lambda row: row[0])
    names = [row[0] for row in rows]
    if len(names) != len(set(names)):
        raise SystemExit("duplicate ghostty theme names after normalization")
    if len(names) != PINNED_THEME_COUNT:
        raise SystemExit(
            f"expected {PINNED_THEME_COUNT} themes at {PINNED_SOURCE_COMMIT}, "
            f"found {len(names)}"
        )
    lines = [LICENSE_HEADER.rstrip("\n")]
    for name, background, foreground, colors in rows:
        lines.append("\t".join([name, background, foreground, *colors]))
    lines.append("")
    return "\n".join(lines)


def verify_tsv(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    if not text.startswith(LICENSE_HEADER):
        raise SystemExit(f"{path}: missing or altered required license header")
    data_lines = [line for line in text[len(LICENSE_HEADER) :].splitlines() if line]
    if not data_lines:
        raise SystemExit(f"{path}: no data rows")
    names: list[str] = []
    for line in data_lines:
        if not DATA_RE.match(line):
            raise SystemExit(f"{path}: malformed row: {line[:80]!r}")
        names.append(line.split("\t", 1)[0])
        if not names[-1].startswith("ghostty-"):
            raise SystemExit(f"{path}: missing ghostty- prefix: {names[-1]}")
    if len(names) != len(set(names)):
        raise SystemExit(f"{path}: duplicate names")
    if names != sorted(names):
        raise SystemExit(f"{path}: rows are not sorted by name")
    if len(names) != PINNED_THEME_COUNT:
        raise SystemExit(
            f"{path}: expected {PINNED_THEME_COUNT} themes, found {len(names)}"
        )
    print(f"OK: {path} has {len(names)} unique sorted ghostty themes")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source-dir",
        type=Path,
        help="Directory of Ghostty theme files (iTerm2-Color-Schemes/ghostty)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Path to write ghostty_themes.tsv",
    )
    parser.add_argument(
        "--verify",
        type=Path,
        help="Validate an existing TSV and exit",
    )
    args = parser.parse_args(argv)

    if args.verify is not None:
        verify_tsv(args.verify)
        return 0

    if args.source_dir is None or args.output is None:
        parser.error("either --verify PATH, or both --source-dir and --output, are required")

    if not args.source_dir.is_dir():
        raise SystemExit(f"source directory not found: {args.source_dir}")

    text = generate(args.source_dir)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(text, encoding="utf-8")
    verify_tsv(args.output)
    print(f"Wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
