# DebClean

Clone di Stacer in Rust per Debian, con interfaccia grafica (egui): barra laterale scura, dashboard con indicatori circolari, tessere colorate e le schede del Deblinux Toolkit. Multilingua: Italiano, English, Español, Français, Deutsch.

## Schede

**Dashboard**: stato del sistema, "Ottimizza ora", accessi rapidi, CPU / RAM / swap / disco con storico a 60 secondi

**Pulizia** (con sottoschede)
- Pulizia: file temporanei, cache APT, pacchetti orfani, browser (cache, cronologia, cookie), cache RAM, log, cache utente, cestino
- Pulizia avanzata: cartelle vuote e file da 0 byte nella home, con elenco, selezione e conferma prima di eliminare
- Risparmio energetico (solo notebook): Bilanciato, Performance, Powersave, Origine, con spiegazioni per principianti. Su un PC fisso la scheda lo segnala e non fa nulla
- Servizi di sistema: verifica stato, avvia, arresta
- Ottimizzazione performance: analisi avvio, swappiness, TRIM, processi pesanti, ottimizza tutto
- Log sistema: ultimi eventi di journalctl

**Fixer**: controlli automatici con "Correggi" + riparazione APT, dipendenze, chiavi, integrità

**Aggiornamenti**: cerca e installa gli aggiornamenti dei pacchetti APT e delle app Flatpak (solo APT, solo Flatpak o tutto insieme), con avviso se serve riavviare

**Sistema**
- Info hardware: Info / Hardware e rete / Memoria RAM (marca, tipo e velocità dei moduli, con dmidecode) / Scheda video (modello, driver, memoria video, monitor) / Tutti i componenti (scheda madre, audio, rete, dischi, USB, input, webcam, monitor, batteria) / About
- Disco: tipo di disco (SSD, NVMe, HDD, USB...), analisi SMART, file grandi, spazio, stress test
- Processi, App installate (APT/Flatpak), Ottimizza RAM (zRAM)
- Temperature, con pulsante "Modalità di raffreddamento"
- Rete: diagnosi gateway/DNS/Internet e ripristino

**Amministratore**
- Kernel manager, governor CPU, swappiness, GRUB (con **Anteprima** delle modifiche), recovery
- Repository e driver: Driver manager che rileva l'hardware e propone i pacchetti
- Startup, permessi utente

**Sicurezza e dati**: firewall UFW, sicurezza avanzata, backup, file shredder, log debug, impostazioni (lingua inclusa)

**Spiegazioni per principianti**: ogni scheda importante ha in alto un riquadro blu che spiega in breve a cosa serve e come usarla.

**Errori di sistema**: in Log debug (e nel Log sistema) gli errori noti del journal vengono spiegati e accompagnati da una soluzione. Le regole sono in `src/solutions.rs`.

**Chiusura**: premendo la X in alto a destra il programma chiede conferma e, se c'è un'operazione in corso (pulizia, aggiornamento...), avvisa prima di chiudere.

**Guida**: spiega in linguaggio semplice tutte le funzioni di DebClean, con avvertenze e ricerca. Per aggiungere o modificare un testo della guida vedi `src/pages_guide.rs`, poi rigenera le traduzioni come descritto sotto

## Compilare

Installa la toolchain con rustup:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Dipendenze di sviluppo:

    sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
        libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev

Build ed esecuzione:

    cargo build --release
    ./target/release/debclean

Per compatibilità con tutte le Debian compila sulla versione più vecchia che vuoi supportare.

## Lingue

All'avvio DebClean usa la lingua del sistema (variabile `LANG`); si cambia in Impostazioni → Lingua.

Le traduzioni stanno in `lang/strings.tsv` (incorporato nel programma): italiano, English, Español, Français, Deutsch.
Per correggerle o aggiungere una lingua:

1. modifica `lang/strings.src` (formato `italiano ::: English ::: Español ::: Français ::: Deutsch`)
2. esegui, dalla cartella del progetto:

       python3 tools/extract_keys.py     # rigenera l'elenco delle frasi (lang/keys.txt)
       python3 tools/build_strings.py    # rigenera lang/strings.tsv e segnala frasi mancanti

3. per una nuova lingua aggiungi una colonna e una voce in `LANGS` in `src/i18n.rs`

Le frasi senza traduzione vengono mostrate in italiano. I messaggi di registro e l'output dei comandi di sistema restano nella lingua originale.

## Pacchetti opzionali usati a runtime

`policykit-1` / `pkexec`, `zenity` (selettore di file e cartelle del File shredder), `dmidecode` (marca e tipo della RAM), `mesa-utils` (informazioni OpenGL), `x11-xserver-utils` (xrandr, monitor), `pciutils`, `usbutils`, `smartmontools`, `ufw`, `timeshift`, `flatpak`, `lm-sensors`, `rsync`, `power-profiles-daemon` (risparmio energetico).

## Sicurezza

La password di root non viene mai salvata: le operazioni privilegiate passano da `pkexec` (Polkit). Le operazioni distruttive chiedono sempre conferma.
