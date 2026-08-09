#!/usr/bin/env python3
"""Validate a Zet publish issue and update the trusted registry index."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = Path(os.environ.get("ZET_REGISTRY_INDEX", ROOT / "registry" / "index.json"))
NAME_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
VERSION_RE = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)
GITHUB_RE = re.compile(
    r"^https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)\.git$"
)
MARKER_RE = re.compile(r"<!-- zet-publish-v1\s*(.*?)\s*-->", re.DOTALL)
MAX_PACKAGE_FILES = 2_000
MAX_PACKAGE_BYTES = 50 * 1024 * 1024


def fail(message: str) -> None:
    raise ValueError(message)


def run(*args: str, cwd: Path | None = None) -> str:
    result = subprocess.run(
        args,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"Komut basarisiz ({' '.join(args[:2])}): {detail}")
    return result.stdout.strip()


def parse_request(event_path: Path) -> tuple[dict[str, str], str]:
    event = json.loads(event_path.read_text(encoding="utf-8"))
    issue = event.get("issue") or {}
    body = issue.get("body") or ""
    actor = ((issue.get("user") or {}).get("login") or "").strip()
    match = MARKER_RE.search(body)
    if not match:
        fail("zet-publish-v1 isaretli JSON istegi bulunamadi.")
    request = json.loads(match.group(1))
    required = {"name", "version", "description", "git", "commit"}
    if set(request) != required or not all(isinstance(request[key], str) for key in required):
        fail("Yayin istegi alanlari gecersiz.")
    if not actor:
        fail("GitHub issue sahibi belirlenemedi.")
    return request, actor


def semver_key(version: str) -> tuple:
    match = VERSION_RE.fullmatch(version)
    if not match:
        fail(f"Gecersiz SemVer: {version}")
    major, minor, patch = (int(match.group(index)) for index in range(1, 4))
    prerelease = match.group(4)
    if prerelease is None:
        return major, minor, patch, 1, ()
    parts = prerelease.split(".")
    if any(part.isdigit() and len(part) > 1 and part.startswith("0") for part in parts):
        fail(f"Gecersiz SemVer on-surumu: {version}")
    identifiers = tuple((0, int(part)) if part.isdigit() else (1, part) for part in parts)
    return major, minor, patch, 0, identifiers


def safe_entry(value: str) -> PurePosixPath:
    if not value or "\\" in value:
        fail("Paket entry alani POSIX biciminde goreli bir yol olmalidir.")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts:
        fail("Paket entry alani depo disina cikamaz.")
    return path


def package_checksum(root: Path) -> str:
    files: list[tuple[str, Path]] = []
    for current, directories, names in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        kept_directories = []
        for directory in directories:
            path = current_path / directory
            if directory == ".git":
                continue
            if path.is_symlink():
                fail(f"Paket sembolik baglanti iceremez: {path.relative_to(root)}")
            kept_directories.append(directory)
        directories[:] = kept_directories
        for name in names:
            if name == ".zet-package":
                continue
            path = current_path / name
            if path.is_symlink():
                fail(f"Paket sembolik baglanti iceremez: {path.relative_to(root)}")
            relative = path.relative_to(root).as_posix()
            files.append((relative, path))

    if len(files) > MAX_PACKAGE_FILES:
        fail(f"Paket en fazla {MAX_PACKAGE_FILES} dosya icerebilir.")
    total_size = sum(path.stat().st_size for _, path in files)
    if total_size > MAX_PACKAGE_BYTES:
        fail("Paket checkout boyutu 50 MiB sinirini asiyor.")

    digest = hashlib.sha256()
    for relative, path in sorted(files):
        relative_bytes = relative.encode("utf-8")
        content = path.read_bytes()
        digest.update(len(relative_bytes).to_bytes(8, "little"))
        digest.update(relative_bytes)
        digest.update(len(content).to_bytes(8, "little"))
        digest.update(content)
    return f"sha256:{digest.hexdigest()}"


def validate_checkout(request: dict[str, str], actor: str) -> tuple[str, str]:
    name = request["name"]
    version = request["version"]
    description = request["description"]
    git = request["git"]
    commit = request["commit"].lower()

    if not NAME_RE.fullmatch(name):
        fail(f"Gecersiz paket adi: {name}")
    semver_key(version)
    if (
        len(description) > 160
        or "-->" in description
        or any(ord(character) < 32 and character != "\t" for character in description)
    ):
        fail("Paket aciklamasi en fazla 160 yazdirilabilir karakter olmalidir.")
    local_test = os.environ.get("ZET_REGISTRY_ALLOW_LOCAL_TEST") == "1" and git.startswith("file://")
    remote_match = GITHUB_RE.fullmatch(git)
    if not local_test:
        if not remote_match:
            fail("Registry yalnizca standart GitHub HTTPS depolarini kabul eder.")
        if remote_match.group(1).lower() != actor.lower():
            fail("Ilk v0.6.5 surumunde depo sahibi ile GitHub issue sahibi ayni olmalidir.")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        fail("Commit kimligi 40 karakterlik Git SHA-1 olmalidir.")

    selected_tag = None
    for tag in (f"v{version}", version):
        references = run(
            "git",
            "ls-remote",
            "--tags",
            git,
            f"refs/tags/{tag}",
            f"refs/tags/{tag}^{{}}",
        )
        hashes = {line.split()[0].lower() for line in references.splitlines() if line.split()}
        if commit in hashes:
            selected_tag = tag
            break
    if selected_tag is None:
        fail("SemVer etiketi istenen commit'i gostermiyor.")

    with tempfile.TemporaryDirectory(prefix="zet-registry-") as temporary:
        checkout = Path(temporary) / "package"
        run(
            "git",
            "-c",
            "core.autocrlf=false",
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--single-branch",
            "--branch",
            selected_tag,
            git,
            str(checkout),
        )
        actual_commit = run("git", "-C", str(checkout), "rev-parse", "HEAD").lower()
        if actual_commit != commit:
            fail("Checkout commit'i istekle uyusmuyor.")

        manifest_path = checkout / "zet.toml"
        if not manifest_path.is_file():
            fail("Depo kokunde zet.toml bulunamadi.")
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        package = manifest.get("package") or {}
        if package.get("name") != name:
            fail("zet.toml paket adi istekle uyusmuyor.")
        if package.get("version") != version:
            fail("zet.toml paket surumu istekle uyusmuyor.")
        if str(package.get("description", "")) != description:
            fail("zet.toml paket aciklamasi istekle uyusmuyor.")
        entry = safe_entry(str(package.get("entry", "src/main.zt")))
        if not (checkout / Path(*entry.parts)).is_file():
            fail(f"Paket entry dosyasi bulunamadi: {entry}")
        checksum = package_checksum(checkout)
    return checksum, git


def update_registry(
    request: dict[str, str], actor: str, checksum: str, git: str
) -> None:
    registry = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
    if registry.get("schema") != 1 or not isinstance(registry.get("packages"), dict):
        fail("Registry index semasi gecersiz.")
    packages = registry["packages"]
    name = request["name"]
    version = request["version"]
    current = packages.get(name)
    if current:
        if current.get("owner", "").lower() != actor.lower():
            fail("Paket adi baska bir GitHub kullanicisi tarafindan kaydedilmis.")
        if current.get("git") != git:
            fail("Kayitli paketin Git deposu degistirilemez.")
    else:
        current = {
            "owner": actor,
            "git": git,
            "description": request["description"],
            "latest": version,
            "versions": {},
        }
        packages[name] = current

    versions = current.setdefault("versions", {})
    release = {"commit": request["commit"].lower(), "checksum": checksum}
    if version in versions and versions[version] != release:
        fail("Ayni paket surumu farkli commit veya checksum ile yeniden yayinlanamaz.")
    versions[version] = release
    current["description"] = request["description"]
    current["latest"] = max(versions, key=semver_key)

    REGISTRY_PATH.write_text(
        json.dumps(registry, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main() -> None:
    if len(sys.argv) != 2:
        fail("Kullanim: registry_publish.py <github-event.json>")
    request, actor = parse_request(Path(sys.argv[1]))
    checksum, git = validate_checkout(request, actor)
    update_registry(request, actor, checksum, git)
    print(
        f"ONAY: {request['name']} v{request['version']} kayda hazir "
        f"({request['commit']}, {checksum})."
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:  # GitHub Action'a tek ve okunabilir hata ver.
        print(f"RED: {error}", file=sys.stderr)
        raise SystemExit(1)
