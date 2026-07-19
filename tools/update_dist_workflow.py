"""Generate cargo-dist CI and apply the repository's least-privilege hardening."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


WORKFLOW = Path(".github/workflows/release.yml")
DIST_CONFIG = Path("dist-workspace.toml")


def read_utf8(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def write_utf8(path: Path, content: str) -> None:
    path.write_bytes(content.encode("utf-8"))


def harden_workflow(content: str) -> str:
    newline = "\r\n" if "\r\n" in content else "\n"
    normalized = content.replace("\r\n", "\n")

    root_permissions = 'permissions:\n  "contents": "write"\n'
    if normalized.count(root_permissions) != 1:
        raise ValueError("cargo-dist output no longer has the expected root permissions")
    normalized = normalized.replace(
        root_permissions,
        'permissions:\n  "contents": "read"\n',
        1,
    )

    host_marker = "  host:\n    needs:\n"
    if normalized.count(host_marker) != 1:
        raise ValueError("cargo-dist output no longer has the expected host job")
    normalized = normalized.replace(
        host_marker,
        '  host:\n    permissions:\n      "contents": "write"\n    needs:\n',
        1,
    )

    host_needs = (
        "    needs:\n"
        "      - plan\n"
        "      - build-local-artifacts\n"
        "      - build-global-artifacts\n"
    )
    if normalized.count(host_needs) != 1:
        raise ValueError("cargo-dist output no longer has the expected host dependencies")
    normalized = normalized.replace(
        host_needs,
        host_needs + "      - custom-release-tag-check\n",
        1,
    )

    host_condition = "    if: ${{ always() && needs.plan.result == 'success'"
    if normalized.count(host_condition) != 1:
        raise ValueError("cargo-dist output no longer has the expected host condition")
    normalized = normalized.replace(
        host_condition,
        "    if: ${{ always() && needs.custom-release-tag-check.result == 'success' && needs.plan.result == 'success'",
        1,
    )

    build_token_marker = "      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n      BUILD_MANIFEST_NAME:"
    build_token_count = normalized.count(build_token_marker)
    if build_token_count != 2:
        raise ValueError(
            "cargo-dist output changed the build-job token layout "
            f"(expected 2, found {build_token_count})"
        )
    normalized = normalized.replace(
        build_token_marker,
        "      BUILD_MANIFEST_NAME:",
    )

    dist_commands = {
        ' --output-format=json > plan-dist-manifest.json':
            ' --allow-dirty --output-format=json > plan-dist-manifest.json',
        ' --print=linkage --output-format=json':
            ' --print=linkage --allow-dirty --output-format=json',
        ' --output-format=json "--artifacts=global"':
            ' --allow-dirty --output-format=json "--artifacts=global"',
        ' --steps=upload --steps=release --output-format=json':
            ' --allow-dirty --steps=upload --steps=release --output-format=json',
    }
    for original, hardened_command in dist_commands.items():
        if normalized.count(original) != 1:
            raise ValueError(f"cargo-dist output no longer has expected command: {original}")
        normalized = normalized.replace(original, hardened_command, 1)

    hardened = normalized.replace("\n", newline)
    if '  "contents": "write"' not in hardened:
        raise ValueError("hardened workflow has no release write permission")
    if '  "contents": "read"' not in hardened:
        raise ValueError("hardened workflow has no root read permission")
    return hardened


def generate(repository_root: Path) -> None:
    config_path = repository_root / DIST_CONFIG
    config_before = read_utf8(config_path)
    newline = "\r\n" if "\r\n" in config_before else "\n"
    normalized = config_before.replace("\r\n", "\n")
    allow_dirty = 'allow-dirty = ["ci"]\n'
    if normalized.count(allow_dirty) != 1:
        raise ValueError("dist config must contain exactly one controlled ci exception")

    write_utf8(config_path, normalized.replace(allow_dirty, "", 1).replace("\n", newline))
    try:
        subprocess.run(["dist", "generate"], check=True, cwd=repository_root)
    finally:
        write_utf8(config_path, config_before)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify generated and hardened output without changing the worktree",
    )
    args = parser.parse_args()

    repository_root = Path(__file__).resolve().parents[1]
    workflow_path = repository_root / WORKFLOW
    before = read_utf8(workflow_path) if args.check else None
    try:
        generate(repository_root)
        hardened = harden_workflow(read_utf8(workflow_path))
        if args.check:
            if hardened != before:
                print(f"{workflow_path} is out of date", file=sys.stderr)
                return 1
            print(f"{workflow_path} is up to date")
        else:
            write_utf8(workflow_path, hardened)
            print(f"generated and hardened {workflow_path}")
        return 0
    finally:
        if args.check and before is not None:
            write_utf8(workflow_path, before)


if __name__ == "__main__":
    raise SystemExit(main())
