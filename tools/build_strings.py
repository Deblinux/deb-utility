#!/usr/bin/env python3
"""Costruisce lang/strings.tsv da lang/strings.src, usando lang/keys.txt come elenco delle chiavi.
Formato strings.src:  italiano ::: English ::: Espanol ::: Francais ::: Deutsch
Uso (dalla cartella del progetto):  python3 tools/extract_keys.py && python3 tools/build_strings.py"""
import re
keys = [k for k in open('lang/keys.txt', encoding='utf-8').read().split('\n') if k != '']
src = {}
for line in open('lang/strings.src', encoding='utf-8').read().split('\n'):
    if not line.strip() or line.startswith('#'):
        continue
    parts = [p.strip() for p in line.split(' ::: ')]
    if len(parts) != 5:
        print('RIGA NON VALIDA:', line[:80]); continue
    src[parts[0].strip()] = parts[1:]
out = ['# Generato da tools/build_strings.py: italiano<TAB>en<TAB>es<TAB>fr<TAB>de']
missing, bad = [], []
for k in keys:
    t = src.get(k.strip())
    if not t:
        missing.append(k); continue
    lead = k[:len(k) - len(k.lstrip())]
    trail = k[len(k.rstrip()):]
    cols = [lead + x + trail for x in t]
    n = k.count('{}')
    for lang, c in zip(['en', 'es', 'fr', 'de'], cols):
        if c.count('{}') != n:
            bad.append((lang, k))
    out.append('\t'.join([k] + cols))
unused = [k for k in src if k not in {x.strip() for x in keys}]
open('lang/strings.tsv', 'w', encoding='utf-8').write('\n'.join(out) + '\n')
print(len(out) - 1, 'voci scritte su', len(keys), 'chiavi')
print('SENZA TRADUZIONE:'); [print('  ', repr(m)) for m in missing]
print('SEGNAPOSTO DIVERSI:'); [print('  ', b) for b in bad]
print('IN strings.src MA NON USATE:'); [print('  ', repr(u)) for u in unused]
