#!/usr/bin/env python3
"""Record exact-lock npm evidence and authentic available notices; no invented grants."""
import hashlib,json,pathlib,shutil
root=pathlib.Path(__file__).resolve().parents[1]
allrows=[]
for name,lockfile,installed in [('v1','fixtures/own/package-lock.json','.qa/v1'),('schema','tests/schema/package-lock.json','.qa/schema')]:
 lock=json.loads((root/lockfile).read_text())
 for key,p in lock['packages'].items():
  if not key:continue
  src=root/installed/key;files=[]
  if src.exists():
   for f in sorted(src.iterdir()):
    if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE','COPYRIGHT')):
     dest=root/'licenses/npm'/name/key.removeprefix('node_modules/')/f.name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,dest);files.append(str(dest.relative_to(root)))
  allrows.append(dict(scope=name,path=key,version=p['version'],integrity=p.get('integrity'),resolved=p.get('resolved'),declared_license=p.get('license'),notices=files,missing_authentic_notice=not files))
(root/'licenses/LOCKED-NPM.json').write_text(json.dumps(allrows,indent=2)+'\n')
for name in ['@raycast/api','@raycast/utils']:
 src=root/'.qa/v1/node_modules'/name;dst=root/'fixtures/locked-deps'/name;dst.mkdir(parents=True,exist_ok=True)
 shutil.copyfile(src/'package.json',dst/'package.json')
 if name=='@raycast/api':shutil.copyfile(src/'types/index.d.ts',dst/'index.d.ts')
print('Exact-lock npm packages:',len(allrows),'missing standalone authentic notices:',sum(p['missing_authentic_notice'] for p in allrows))
