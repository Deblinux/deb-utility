use crate::i18n::t;
use crate::{ui, App};
use eframe::egui;

/// (titolo, spiegazione, avvertenza: stringa vuota se non serve)
type Entry = (&'static str, &'static str, &'static str);

// GUIDE-BEGIN
const GUIDE: &[(&str, &[Entry])] = &[
    (
        "PANORAMICA",
        &[(
            "Dashboard",
            "È la schermata iniziale. Mostra in un colpo d'occhio se il sistema è in buono stato, l'uso di CPU, RAM, swap e disco e il loro storico degli ultimi 60 secondi. «Ottimizza ora» esegue la pulizia con le voci già selezionate; le tessere portano alle funzioni principali.",
            "",
        )],
    ),
    (
        "MANUTENZIONE",
        &[
            (
                "Pulizia",
                "Libera spazio eliminando i file inutili: temporanei, cache di APT e dei browser, pacchetti orfani, log vecchi, cache utente e cestino. Spunta le voci, premi «Analizza» per vedere quanto spazio recuperi e poi «Pulisci selezionati». Le operazioni da amministratore chiedono la password una volta sola.",
                "La cancellazione di cronologia e cookie dei browser è irreversibile: chiudi prima i browser.",
            ),
            (
                "Pulizia avanzata",
                "Cerca nella tua cartella personale le cartelle vuote e i file da 0 byte. Vedi l'elenco completo, scegli cosa eliminare e confermi prima che venga cancellato qualcosa.",
                "Alcuni programmi usano file vuoti di proposito: controlla l'elenco prima di confermare.",
            ),
            (
                "Pianificatore di manutenzione",
                "Crea timer e servizi systemd a livello utente che operano in background senza tenere aperto DebClean. Esegue periodicamente la pulizia della cache utente e il trimming dell'SSD, mantenendo il sistema veloce e pulito a costo zero di RAM.",
                "Disattivando il timer la manutenzione tornerà a essere esclusivamente manuale.",
            ),
            (
                "Risparmio energetico",
                "Solo per i notebook: sceglie come il processore usa l'energia. Bilanciato è per l'uso di tutti i giorni, Performance dà la massima velocità consumando di più, Powersave fa durare di più la batteria, Origine ripristina le impostazioni precedenti. Su un PC fisso la funzione non è disponibile.",
                "Le impostazioni valgono fino al riavvio del computer.",
            ),
            (
                "Servizi di sistema",
                "I servizi sono programmi che girano sempre in background (rete, SSH, Bluetooth, firewall...). Puoi verificare se un servizio è attivo, avviarlo o arrestarlo per la sessione corrente.",
                "Fermare servizi critici come networking o NetworkManager può togliere la connessione a Internet.",
            ),
            (
                "Ottimizzazione performance",
                "Raccoglie gli strumenti per velocizzare il computer: analisi dell'avvio, regolazione della swappiness, TRIM per gli SSD, elenco dei processi più pesanti e «Ottimizza tutto» per una manutenzione completa in un colpo solo.",
                "Alcune operazioni richiedono i privilegi di amministratore.",
            ),
            (
                "Log sistema",
                "Mostra gli ultimi eventi e messaggi di sistema registrati da journalctl, con filtri per errori, avvio corrente e kernel. Serve a capire cosa è andato storto quando qualcosa non funziona.",
                "",
            ),
            (
                "Fixer",
                "Controlla lo stato di salute del sistema (pacchetti, dipendenze, servizi, spazio, orologio...) e per ogni problema offre il pulsante «Correggi». «Correggi tutto» applica tutte le correzioni insieme; gli strumenti manuali riparano il database APT, le dipendenze e le chiavi e verificano l'integrità dei pacchetti.",
                "Le correzioni modificano pacchetti e servizi: leggi il problema prima di applicarle.",
            ),
            (
                "Aggiornamenti",
                "Controlla se ci sono aggiornamenti per i pacchetti APT (il sistema Debian e i suoi programmi) e per le app Flatpak. Puoi aggiornare solo APT, solo Flatpak o tutto con un clic.",
                "Durante l'aggiornamento non spegnere il computer: alla fine potrebbe servire un riavvio.",
            ),
        ],
    ),
    (
        "SISTEMA",
        &[
            (
                "Info hardware",
                "Sei schede: Info (sistema, memoria, dischi, rete), Hardware e rete (processore, scheda madre, modelli dei dischi, gateway), Memoria RAM (marca, tipo e velocità di ogni modulo), Scheda video (modello, driver, memoria video, monitor), Tutti i componenti (elenco completo del computer) e About. Serve a sapere esattamente che computer hai.",
                "",
            ),
            (
                "Disco",
                "Mostra il tipo di ogni disco (SSD, NVMe, HDD, USB). Analisi disco controlla la salute con SMART, File grandi trova i file pesanti e inutilizzati, Spazio disco calcola la dimensione delle cartelle e Stress test mette sotto sforzo CPU e RAM per scoprire instabilità.",
                "Lo stress test porta il computer al limite: salva il lavoro prima di avviarlo.",
            ),
            (
                "Processi",
                "Elenca i programmi in esecuzione con la CPU e la memoria che usano. «Termina» chiede al programma di chiudersi, «Kill» lo chiude con la forza se non risponde.",
                "Chiudere un processo di sistema può rendere instabile il computer.",
            ),
            (
                "App installate",
                "Elenca i pacchetti APT e le app Flatpak installati e permette di disinstallarli. Usa la ricerca per trovare subito quello che cerchi.",
                "La disinstallazione chiede conferma ma non si può annullare.",
            ),
            (
                "Container (Docker / Podman)",
                "Permette di monitorare i container in esecuzione o fermi, leggere gli ultimi log, avviarli, arrestarli ed eseguire «Prune» per liberare lo spazio occupato da immagini orfane e volumi abbandonati.",
                "Il Prune elimina definitivamente i container fermi e le immagini non associate ad alcun contenitore.",
            ),
            (
                "Ottimizza RAM",
                "Crea una zRAM, cioè una memoria compressa dentro la RAM, per evitare rallentamenti quando la memoria scarseggia. Scegli la dimensione con il cursore e premi «Ottimizza RAM»; «Disattiva zRAM» la rimuove.",
                "La configurazione vale fino al riavvio.",
            ),
            (
                "Temperature",
                "Mostra le temperature dei sensori. Il pulsante «Attiva modalità di raffreddamento» limita la velocità del processore per produrre meno calore e far girare più piano le ventole; «Disattiva e ripristina» torna alla velocità normale.",
                "",
            ),
            (
                "Rete",
                "Controlla con tre spie il gateway (router), il DNS e la connessione a Internet. «Ripristina rete e Wi-Fi» riavvia NetworkManager per risolvere i problemi di connessione.",
                "La connessione cade per qualche secondo.",
            ),
        ],
    ),
    (
        "AMMINISTRATORE",
        &[
            (
                "Kernel, Grub e Recovery",
                "Quattro schede: Kernel manager rimuove i kernel vecchi (quello in uso è protetto), Kernel e prestazioni regola governor e swappiness, Grub modifica i parametri di avvio con un'anteprima prima di salvare, Recovery offre strumenti di emergenza (audio, rete, ricostruzione e reinstallazione di GRUB, Flathub).",
                "Modificare GRUB può impedire l'avvio del computer: usa sempre l'anteprima e tieni un backup.",
            ),
            (
                "Repository e driver",
                "Driver manager rileva l'hardware e propone i driver e i firmware da installare. App repository mostra e aggiunge le sorgenti software, Repository Manager abilita i Debian Backports e aggiorna gli elenchi dei pacchetti.",
                "Aggiungi solo repository di cui ti fidi.",
            ),
            (
                "Startup",
                "Elenca le applicazioni che partono automaticamente al login. Togli la spunta a quelle che non ti servono per ridurre i tempi di avvio; puoi riattivarle quando vuoi.",
                "",
            ),
            (
                "Permessi utente",
                "Abilita o disabilita l'accesso del tuo utente a funzioni speciali: amministratore (sudo), porte seriali per Arduino e USB, VirtualBox e Docker.",
                "Servono un logout o un riavvio perché le modifiche diventino attive.",
            ),
            (
                "Gestore Pacchetti",
                "Cerca pacchetti nei repository ufficiali APT, visualizza lo stato (installato o disponibile) e la descrizione dettagliata. Permette di installare, rimuovere o purgare qualsiasi pacchetto con un clic, sfruttando i permessi di amministrazione in sicurezza.",
                "La rimozione o la purga di pacchetti di sistema essenziali può compromettere il funzionamento del sistema operativo.",
            ),
        ],
    ),
    (
        "SICUREZZA E DATI",
        &[
            (
                "Sicurezza e firewall",
                "Firewall UFW attiva o disattiva la protezione contro gli accessi non autorizzati dalla rete. Sicurezza avanzata elenca i servizi in ascolto, scansiona le porte locali e mostra i log del firewall.",
                "Disattivare il firewall espone di più il PC alla rete.",
            ),
            (
                "Backup",
                "File e cartelle copia i tuoi dati in una destinazione a scelta. Pacchetti di sistema salva l'elenco dei pacchetti e dei repository e li ripristina dopo una reinstallazione. Punti di ripristino crea e ripristina istantanee dell'intero sistema con Timeshift.",
                "Ripristinare un punto riporta il sistema indietro: le modifiche successive vanno perse.",
            ),
            (
                "File shredder",
                "Sovrascrive più volte un file o una cartella con dati casuali e poi li elimina, in modo che non siano recuperabili. Scegli il numero di passaggi.",
                "È irreversibile; su SSD e file system con snapshot la sovrascrittura non dà garanzie complete.",
            ),
            (
                "Log debug",
                "Mostra le operazioni eseguite da DebClean e gli errori del journal di sistema; se trova errori noti ti suggerisce come risolverli. Puoi mettere in pausa il registro o pulirlo.",
                "",
            ),
            (
                "Impostazioni",
                "Qui scegli la lingua dell'interfaccia e attivi la pulizia rapida della cache utente all'avvio. DebClean non salva la password di root: usa pkexec e il sistema chiede l'autorizzazione quando serve.",
                "",
            ),
        ],
    ),
    (
        "AIUTO",
        &[(
            "Guida",
            "Questa schermata spiega ogni funzione di DebClean. Usa la ricerca per trovare subito l'argomento che ti interessa.",
            "",
        )],
    ),
];
// GUIDE-END

impl App {
    pub(crate) fn ui_guide(&mut self, ui: &mut egui::Ui) {
        ui::title(ui, "Guida di DebClean");
        ui::card(ui, |ui| {
            ui.label(egui::RichText::new(t("DebClean riunisce in un'unica applicazione pulizia, riparazione, monitoraggio e amministrazione del sistema. Quando una funzione richiede i privilegi di amministratore, il sistema mostra una finestra per l'autorizzazione; le operazioni delicate chiedono sempre conferma prima di procedere. Qui sotto trovi la spiegazione di ogni funzione.")).size(17.0));
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(t("Cerca nella guida:"));
            ui.text_edit_singleline(&mut self.guide_filter);
        });

        let q = self.guide_filter.trim().to_lowercase();
        let searching = !q.is_empty();
        let hit = |e: &Entry| -> bool {
            !searching
                || t(e.0).to_lowercase().contains(&q)
                || t(e.1).to_lowercase().contains(&q)
                || t(e.2).to_lowercase().contains(&q)
        };

        let mut shown = 0;
        for (group, entries) in GUIDE.iter() {
            let hits: Vec<&Entry> = entries.iter().filter(|e| hit(e)).collect();
            if hits.is_empty() {
                continue;
            }
            ui.add_space(12.0);
            ui.label(egui::RichText::new(t(*group)).size(14.0).strong().color(ui::TEXT_DIM));
            ui.add_space(2.0);
            for e in hits {
                shown += 1;
                ui::card(ui, |ui| {
                    egui::CollapsingHeader::new(egui::RichText::new(t(e.0)).strong().size(19.0))
                        .open(if searching { Some(true) } else { None })
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(t(e.1)).size(17.0));
                            if !e.2.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(format!("{}: {}", t("Attenzione"), t(e.2)))
                                        .color(ui::WARN),
                                );
                            }
                        });
                });
                ui.add_space(6.0);
            }
        }
        if shown == 0 {
            ui.add_space(8.0);
            ui.label(t("Nessun risultato per questa ricerca."));
        }
    }
}
