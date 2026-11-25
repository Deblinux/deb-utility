# DebUtility

**DebUtility** è una suite nativa in **Rust** (`eframe` / `egui`) progettata per la pulizia profonda, l'ottimizzazione, la diagnostica e la manutenzione sicura dei sistemi **Debian e Ubuntu**.

Unisce in un'unica applicazione leggera e reattiva funzioni che solitamente richiedono più strumenti separati, offrendo sia un'interfaccia grafica moderna (**Dark Mode** e **Light Mode**) sia una **modalità CLI headless** per script e automazioni.

---

## Caratteristiche principali

### Pulizia e gestione dello spazio
* **Pulizia multi-ecosistema:** Cache APT, pacchetti orfani/residui (`rc`), vecchi log di sistema, cache utente e browser, miniature e file temporanei.
* **Gestione Snap e Flatpak:** Analisi e pulizia dei pacchetti inutilizzati o delle revisioni obsolete.
* **Sviluppo & Container:** Pulizia mirata degli ambienti di sviluppo (Docker, Cargo, npm, pip) per recuperare rapidamente gigabyte occupati da build e cache.
* **Analizzatore Disco:** Esplorazione visuale dello spazio occupato su disco per individuare cartelle e file più pesanti.
* **File duplicati e Residui di sistema:** Rilevamento e rimozione guidata di duplicati e file abbandonati.

### Sicurezza, controllo e ripristino
* **Indicatori visivi del livello di rischio:** Ogni modulo è classificato chiaramente con pallini e badge di stato (**Sicura**, **Con cautela**, **Delicato**) per operare sempre con consapevolezza.
* **Quarantena / Ripristino (Undo):** I file eliminati dai moduli supportati vengono spostati in una quarantena temporanea con scadenza automatica, permettendo il ripristino nella posizione originale in caso di ripensamenti.
* **Snapshot pre-pulizia:** Integrazione con **Timeshift** e **Snapper** per mettere al sicuro il sistema prima di interventi delicati.
* **Gestione Kernel & `/boot`:** Rimozione sicura dei vecchi kernel con protezione automatica del kernel attualmente in esecuzione.
* **Privacy & Telemetry Guard:** Scansione di demoni di telemetria/diagnostica e blocco rapido dei domini di tracciamento noti.
* **Sicurezza, Hardening e File Shredder:** Controlli di sicurezza del sistema, permessi utente e cancellazione sicura dei file sensibili.

### Prestazioni, avvio e diagnostica
* **Gestore Avvio & Autostart Avanzato:** Analisi dei tempi di boot (`systemd-analyze`) e gestione dei programmi all'avvio.
* **Fixer e Storico APT:** Risoluzione guidata dei problemi di pacchetti/dipendenze e consultazione della cronologia delle operazioni APT.
* **Hardware, Temperature e Dischi:** Monitoraggio di temperature, risorse, RAM, rete e stato dei dischi.
* **Pianificatore `systemd`:** Automazione periodica delle operazioni di manutenzione.
* **Report Diagnostico e Guida integrata:** Esportazione di report di sistema (HTML / JSON), supporto **multilingua** completo e guida interna consultabile in qualsiasi momento.

---

## Requisiti e compilazione

### Dipendenze di sistema (Debian / Ubuntu)
Assicurati di avere installati la toolchain **Rust** (`cargo`) e le librerie di sviluppo necessarie per la GUI:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
