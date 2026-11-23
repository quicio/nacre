#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$repo_root" <<'PY'
import json
from pathlib import Path
import subprocess
import sys

repo_root = Path(sys.argv[1]).resolve()
result = subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--locked"],
    cwd=repo_root,
    capture_output=True,
    text=True,
    check=False,
)
if result.returncode != 0:
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    raise SystemExit(result.returncode)

metadata = json.loads(result.stdout)
packages = {package["id"]: package for package in metadata["packages"]}
workspace_ids = set(metadata["workspace_members"])
workspace = {
    packages[package_id]["name"]: package_id for package_id in workspace_ids
}
resolve_nodes = {
    node["id"]: node for node in metadata["resolve"]["nodes"]
}

allowed_workspace_edges = {
    "nacre-domain": set(),
    "nacre-application": {"nacre-domain"},
    "nacre-dsp": {"nacre-domain"},
    "nacre-audio-runtime": {
        "nacre-domain",
        "nacre-application",
        "nacre-dsp",
    },
    "nacre-standalone": {
        "nacre-domain",
        "nacre-application",
        "nacre-dsp",
        "nacre-audio-runtime",
    },
}
forbidden_domain_packages = {"cpal", "egui", "serde"}
errors = set()

if set(workspace) != set(allowed_workspace_edges):
    unexpected = sorted(set(workspace) - set(allowed_workspace_edges))
    missing = sorted(set(allowed_workspace_edges) - set(workspace))
    if unexpected:
        errors.add("unexpected workspace packages: " + ", ".join(unexpected))
    if missing:
        errors.add("missing workspace packages: " + ", ".join(missing))

manifest_directories = {
    Path(packages[package_id]["manifest_path"]).parent.resolve(): name
    for name, package_id in workspace.items()
}
direct_workspace_edges = {name: set() for name in workspace}
direct_package_names = {name: set() for name in workspace}

for name, package_id in workspace.items():
    package = packages[package_id]
    package_directory = Path(package["manifest_path"]).parent.resolve()
    for dependency in package["dependencies"]:
        direct_package_names[name].add(dependency["name"])
        dependency_path = dependency.get("path")
        if dependency_path is None:
            continue
        path = Path(dependency_path)
        if not path.is_absolute():
            path = package_directory / path
        dependency_name = manifest_directories.get(path.resolve())
        if dependency_name is not None:
            direct_workspace_edges[name].add(dependency_name)

    for dependency in resolve_nodes[package_id]["deps"]:
        dependency_id = dependency["pkg"]
        if dependency_id in workspace_ids:
            dependency_name = packages[dependency_id]["name"]
            direct_workspace_edges[name].add(dependency_name)

for name, dependencies in direct_workspace_edges.items():
    unexpected = dependencies - allowed_workspace_edges.get(name, set())
    if unexpected:
        errors.add(
            f"{name} has forbidden workspace dependencies: "
            + ", ".join(sorted(unexpected))
        )

domain_id = workspace["nacre-domain"]
domain_reachable = set()
stack = [dependency["pkg"] for dependency in resolve_nodes[domain_id]["deps"]]
while stack:
    package_id = stack.pop()
    if package_id in domain_reachable:
        continue
    domain_reachable.add(package_id)
    stack.extend(
        dependency["pkg"] for dependency in resolve_nodes[package_id]["deps"]
    )

domain_dependency_names = direct_package_names["nacre-domain"] | {
    packages[package_id]["name"] for package_id in domain_reachable
}
for forbidden in sorted(forbidden_domain_packages & domain_dependency_names):
    errors.add(f"nacre-domain must not depend on {forbidden}")

cpal_ids = {
    package_id
    for package_id, package in packages.items()
    if package["name"] == "cpal"
}
runtime_id = workspace["nacre-audio-runtime"]

for name, dependency_names in direct_package_names.items():
    if name != "nacre-audio-runtime" and "cpal" in dependency_names:
        errors.add(f"{name} must not depend directly on cpal")
    if name != "nacre-audio-runtime" and "rtrb" in dependency_names:
        errors.add(f"{name} must not depend directly on rtrb")
    if name != "nacre-standalone" and ({"egui", "eframe"} & dependency_names):
        found = ", ".join(sorted({"egui", "eframe"} & dependency_names))
        errors.add(f"{name} must not depend directly on {found}")

for name, package_id in workspace.items():
    if package_id == runtime_id:
        continue
    reachable_cpal = set()
    visited = {package_id}
    stack = [dependency["pkg"] for dependency in resolve_nodes[package_id]["deps"]]
    while stack:
        current = stack.pop()
        if current == runtime_id or current in visited:
            continue
        visited.add(current)
        if current in cpal_ids:
            reachable_cpal.add(current)
            continue
        stack.extend(
            dependency["pkg"] for dependency in resolve_nodes[current]["deps"]
        )
    if reachable_cpal:
        errors.add(f"{name} reaches cpal outside nacre-audio-runtime")

visit_state = {name: 0 for name in workspace}
visit_path = []


def visit(name):
    if visit_state[name] == 2:
        return
    if visit_state[name] == 1:
        start = visit_path.index(name)
        cycle = visit_path[start:] + [name]
        errors.add("workspace dependency cycle: " + " -> ".join(cycle))
        return

    visit_state[name] = 1
    visit_path.append(name)
    for dependency in sorted(direct_workspace_edges[name]):
        visit(dependency)
    visit_path.pop()
    visit_state[name] = 2


for name in sorted(workspace):
    visit(name)

if errors:
    for error in sorted(errors):
        print(f"error: {error}", file=sys.stderr)
    raise SystemExit(1)

print(
    f"Dependency graph OK: {len(workspace)} workspace packages, "
    "allowed edges, no forbidden domain dependencies, no cycles."
)
PY
