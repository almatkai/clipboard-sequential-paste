#!/usr/bin/env python3
"""Optional parent-machine Git-object cross-check; never reads host work files."""
import pathlib,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
repos=[('host',root.parents[2]/'Tezbar','ea100cee3f69711de12485ba061950c5fc65258f'),('sdk',root.parents[1]/'tezbar-extension-sdk-rust','e955911f0bf5b558f9d3eb5183df0adc95c22af2')]
# This script is explicitly optional: ordinary QA uses frozen, hashed fixtures and
# the HTTPS SDK and requires no sibling checkout. Paths are for review provenance.
for name,repo,rev in repos:
 assert repo.is_dir(),repo
 for file in sorted((root/'fixtures'/name).rglob('*')):
  if not file.is_file():continue
  rel=str(file.relative_to(root/'fixtures'/name));actual=subprocess.check_output(['git','-C',str(repo),'show',rev+':'+rel]);assert actual==file.read_bytes(),(name,rel)
 print(name,'exact pinned Git objects match all fixtures')
