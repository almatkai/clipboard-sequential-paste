#!/usr/bin/env python3
"""Build a local ARM64 development ZIP; no artifact URLs, installation or release."""
import hashlib,json,pathlib,shutil,stat,subprocess,zipfile
root=pathlib.Path(__file__).resolve().parents[1]
qa=root/'.qa';stage=qa/'dev-package';stage.mkdir(exist_ok=True)
def copy(src,dst):
 dst=stage/dst;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dst)
copy(root/'target/release/clipboard-sequential-paste','bin/clipboard-sequential-paste')
(stage/'bin/clipboard-sequential-paste').chmod(0o755)
copy(root/'resources/extension-icon.png','resources/extension-icon.png')
copy(root/'licenses/OWN-LICENSE','LICENSE')
copy(root/'NOTICE','NOTICE')
for file in (root/'licenses').rglob('*'):
 if file.is_file():copy(file,pathlib.Path('resources/licenses')/file.relative_to(root/'licenses'))
m=json.loads((root/'extension.json').read_text())
m['files']={str(p.relative_to(stage)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(stage.rglob('*')) if p.is_file() and p.name!='extension.json'}
(stage/'extension.json').write_text(json.dumps(m,indent=2)+'\n')
archive=qa/'clipboard-sequential-paste-aarch64-apple-darwin-dev.zip'
with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
 for p in sorted(stage.rglob('*')):
  if p.is_file():z.write(p,p.relative_to(stage))
extract=qa/'dev-extracted';extract.mkdir(exist_ok=True)
with zipfile.ZipFile(archive) as z:
 assert not any(n.startswith('/') or '..' in pathlib.PurePosixPath(n).parts for n in z.namelist());z.extractall(extract)
actual={str(p.relative_to(extract)):hashlib.sha256(p.read_bytes()).hexdigest() for p in extract.rglob('*') if p.is_file() and p.name!='extension.json'}
assert actual==m['files'];assert (extract/'resources/extension-icon.png').read_bytes()==(root/'resources/extension-icon.png').read_bytes()
binary=extract/'bin/clipboard-sequential-paste';binary.chmod(0o755)
subprocess.run(['file',str(binary)],check=True)
print('Dev ZIP sha256:',hashlib.sha256(archive.read_bytes()).hexdigest(),'inventory members:',len(actual))
subprocess.run(['python3',str(root/'scripts/process-qa.py'),str(binary)],check=True)
print('Extracted ARM64 production domain process QA passed; source artifacts remain empty; nothing installed')
