#!/usr/bin/env python3
"""Copy authentic locked crate notices from the isolated HTTPS Cargo cache."""
import json,pathlib,re
root=pathlib.Path(__file__).resolve().parents[1]
lock={'package':[{k:json.loads(v) for k,v in re.findall(r'^(name|version|source|checksum) = (".*?")$',part,re.M)} for part in (root/'Cargo.lock').read_text().split('[[package]]')[1:]]};cache=root/'.qa/cargo-home/registry/src'
index=[]
for p in lock['package']:
 if not p.get('source','').startswith('registry+'):continue
 matches=list(cache.glob('*/'+p['name']+'-'+p['version']));assert len(matches)==1,p
 source=matches[0];notices=[]
 for file in sorted(source.rglob('*')):
  if file.is_file() and any(file.name.upper().startswith(x) for x in ['LICENSE','COPYING','NOTICE','COPYRIGHT']) and len(file.relative_to(source).parts)<=3:
   dest=root/'licenses/crates'/f"{p['name']}-{p['version']}"/file.relative_to(source);dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(file.read_bytes());notices.append(str(dest.relative_to(root)))
 section=(source/'Cargo.toml').read_text().split('[package]',1)[1].split('\n[',1)[0];meta={k:json.loads(v) for k,v in re.findall(r'^(license|license-file) = (".*?")$',section,re.M)}
 index.append(dict(name=p['name'],version=p['version'],checksum=p['checksum'],license=meta.get('license'),license_file=meta.get('license-file'),notices=notices,missing_authentic_notice=not notices))
(root/'licenses/LOCKED-CRATES.json').write_text(json.dumps(index,indent=2)+'\n')
print('Authentic crate notices:',len(index),'packages; missing:',[p['name'] for p in index if p['missing_authentic_notice']])
