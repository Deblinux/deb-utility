# DebClean 🧹🐧

**DebClean** è un ottimizzatore, pulitore e gestore di sistema leggero, veloce e nativo scritto interamente in **Rust** per **Debian, Ubuntu e distribuzioni derivate**.

---

## ✨ Caratteristiche principali

* **Pulizia e Manutenzione:** Rimozione sicura di cache utente e di sistema, pacchetti orfani, log obsoleti, file temporanei e ricerca di file di grandi dimensioni.
* **Monitoraggio Sistema e Hardware:** Visualizzazione in tempo reale di CPU, RAM, temperature, dischi, rete, processi attivi e dettagli hardware completi.
* **Amministrazione Avanzata:** Gestione Kernel e GRUB, strumenti di Recovery, controllo servizi all'avvio (Startup), permessi utente e repository/driver.
* **Package Manager Integrato:** Interfaccia grafica ispirata a Synaptic per cercare, ispezionare, installare, rimuovere o purgare pacchetti APT con un clic.
* **Sicurezza e Dati:** Gestione Firewall, Backup, File Shredder (cancellazione sicura), audit di sicurezza e log di debug.
* **Multilingua (i18n):** Interfaccia dinamica disponibile in **Italiano, Inglese, Spagnolo, Francese e Tedesco**.
* **Leggero e Sicuro:** Consumo minimo di RAM e protezione integrata contro l'avvio multiplo accidentale.

---

## 🚀 Compilazione ed Esecuzione

Assicurati di avere **Rust / Cargo** installato sul tuo sistema Debian/Ubuntu:

```bash
git clone [https://github.com/Deblinux/debclean.git](https://github.com/Deblinux/debclean.git)
cd debclean
cargo build --release
./target/release/debclean
