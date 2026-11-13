#!/usr/bin/env python3
"""Estrae dai sorgenti tutte le stringhe traducibili (chiavi italiane) e le scrive in lang/keys.txt."""
import re, glob, sys

LIT = r'"((?:[^"\\]|\\.)*)"'
def unesc(s):
    return s.replace('\\"', '"').replace("\\'", "'").replace('\\\\', '\\')

keys = []
def add(k):
    k = unesc(k)
    if k and k not in keys and re.search(r'[A-Za-zÀ-ÿ]{2,}|^n/d$', k):
        keys.append(k)

files = sorted(glob.glob('src/*.rs'))
for fn in files:
    if fn.endswith('i18n.rs'):
        continue
    s = open(fn).read()
    for m in re.finditer(r'\btr?\(' + LIT + r'\)', s):
        add(m.group(1))
    for m in re.finditer(r'\btf\(\s*' + LIT, s):
        add(m.group(1))
    for m in re.finditer(r'ui::hint\(\s*ui,\s*' + LIT, s):
        add(m.group(1))
    for m in re.finditer(r'ui::title\(ui, ' + LIT, s):
        add(m.group(1))
    for m in re.finditer(r'sub_tabs\(\s*ui,\s*&mut self\.sub,\s*&\[(.*?)\]\s*,?\s*\)', s, re.S):
        for l in re.finditer(LIT, m.group(1)):
            add(l.group(1))
    for m in re.finditer(r'ui::gauge\(\s*ui,\s*[^,]+,\s*' + LIT, s):
        add(m.group(1))
    for m in re.finditer(r'ui::kv\(\s*ui,\s*' + LIT, s):
        add(m.group(1))
    for m in re.finditer(r'\(Page::\w+,\s*' + LIT + r',\s*' + LIT + r'\)', s):
        add(m.group(1)); add(m.group(2))
    for m in re.finditer(r'\(Page::\w+,\s*' + LIT + r'\)', s):
        add(m.group(1))
    for m in re.finditer(r'\(\s*' + LIT + r'\.into\(\),', s):
        add(m.group(1))
    for m in re.finditer(r'\(\s*' + LIT + r'\.into\(\)', s):
        add(m.group(1))

s = open('src/main.rs').read()
for m in re.finditer(r'^\s*\(\s*"([A-ZÀ-Ú ]+)",', s, re.M):
    add(m.group(1))
s = open('src/cleaner.rs').read()
for m in re.finditer(r'mk\(Kind::\w+,\s*' + LIT + r',\s*' + LIT, s):
    add(m.group(1)); add(m.group(2))
s = open('src/health.rs').read()
for m in re.finditer(r'mk\(\s*' + LIT + r',\s*Status::\w+,\s*' + LIT, s):
    add(m.group(1)); add(m.group(2))
for m in re.finditer(r'mk\(\s*' + LIT + r',', s):
    add(m.group(1))
s = open('src/pages_admin.rs').read()
for m in re.finditer(r'push_prop\(\s*&mut props,\s*' + LIT + r',\s*' + LIT, s):
    add(m.group(2))
s = open('src/info.rs').read()
for m in re.finditer(r'\(\s*' + LIT + r',\s*"(?:sys_vendor|product_name|board_name|bios_vendor|bios_version|bios_date)"\)', s):
    add(m.group(1))
s = open('src/pages_sys.rs').read()
a = s.index('fn describe_disk'); b = s.index('fn disk_label')
for m in re.finditer(LIT, s[a:b]):
    if ' ' in m.group(1) or m.group(1).startswith(('SSD', 'HDD', 'Disco', 'Tipo', 'Scheda')):
        add(m.group(1))
s = open('src/pages_clean.rs').read()
for blk in re.finditer(r'const SERVICES.*?\];', s, re.S):
    for l in re.finditer(LIT, blk.group(0)):
        add(l.group(1))
for blk in re.finditer(r'let profiles.*?\n        \];', s, re.S):
    for l in re.finditer(LIT, blk.group(0)):
        if ' ' in l.group(1) and 'sh' not in l.group(1)[:2] and 'if [' not in l.group(1) and 'powerprofilesctl' not in l.group(1):
            add(l.group(1))
for l in re.finditer(r'\(\s*' + LIT + r',\s*format!\(', s):
    add(l.group(1))
for k in ['Cartelle vuote', "Cartelle che non contengono nulla. Eliminarle non fa perdere dati e rende l'archivio più ordinato.", 'File vuoti (0 byte)', 'File che non contengono dati. Spesso sono residui di operazioni interrotte.', "Attenzione: alcuni programmi usano file vuoti di proposito. Controlla l'elenco e deseleziona quelli che vuoi tenere.", 'cartelle vuote verranno eliminate definitivamente:', 'file vuoti verranno eliminati definitivamente:', 'Avvio silenzioso', 'Schermata grafica di avvio', 'Utile solo se hai problemi con la grafica', 'Swap compresso in RAM', 'Consigliato con driver NVIDIA proprietario', "Pulizia rapida della cache utente all'avvio", "Per sicurezza DebClean non salva la password di root: le operazioni privilegiate usano pkexec e il sistema chiede l'autorizzazione tramite la finestra di Polkit."] + ["Performance", "Origine", "In hold: ", " pacchetti rimossi hanno ancora file di configurazione", "Occupato", " — usa la scheda Pulizia per liberare spazio", "In carica", "Scarica", "Carica completa", "Stato sconosciuto", "n/d", "Online", "Offline", "(nessun output)"]:
    add(k)

s = open('src/pages_guide.rs').read()
a = s.index('// GUIDE-BEGIN'); b = s.index('// GUIDE-END')
for m in re.finditer(LIT, s[a:b]):
    add(m.group(1))
s = open('src/solutions.rs').read()
a = s.index('// SOLUTIONS-BEGIN'); b = s.index('// SOLUTIONS-END')
for m in re.finditer(LIT, s[a:b]):
    add(m.group(1))
open('lang/keys.txt', 'w').write('\n'.join(keys) + '\n')
print(len(keys), 'chiavi')
