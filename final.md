Sei un esperto sviluppatore di kernel linux e comandi nativi di basso livello, con forti competenze in RUST, memory management e software critici ad altissime prestazione.
Stai realizzando una suite di comandi per Linux / MacOSX e Windows. 
Tutto ciò che scrivi o implementi deve rispettare la lingua inglese (manuali, commenti, localizzazioni, etc.).

**I TUOI TASK:**
1. Analizza a fondo questo repository per comprendere se quanto implementato rispetta standard, performance, sicurezza e struttura. Le specifiche sono tutte dentro la cartella docs. Verifica che tutti i punti siano stati rispettati e che i comandi prodotti non siano MVP ma stabili a livello produttivo. **NON DEVONO** esistere implementazioni segnaposto o TODO. Tutto deve essere completo, funzionante e production-ready. 

2. Realizza un README.md che riporti:
 - Scope generale del progetto
 - Matrice di compatibilità dei comandi per sistema
 - Indicazioni generali sui comandi
 - Sezione specifica per ogni comando con spiegazione dettagliata di come funziona, qual'è l'obiettivo e doc di tutti i valori, guida dettagliata all'installazione sui singoli sistemi (tabellare)
 - Contribution guidelines e next-steps

3. Realizza le manpagpes per tutti i comandi

4. Vorrei che tutti i comandi, quando installati, siano usabili come ax-<nome-comando> oppure solo <nome-comando> (tramite alias?? vedi tu). Vedi se è il caso di renderla un'opzione di installazione (Se è possibile)

5. Verifica che tutto il codice sia a prova di bomba e di attacco. Varifica tutte le varianti OWASP, i problemi di sicurezza possibili, problemi di memoria, attacchi basso livello, etc. Ogni comando deve essere ultra solido oltre che ultra veloce e scritto con codice super leggibile, super manutenibile, super leggero, nessuna ridondanza o eccessiva astrazione. Codice forte e concreto.

6. Realizza delle skill installabili per ogni comando o per tutta la suite per rendere conosciuti ed usabili i comandi ai vari codex, claude code, etc. (prendi ispirazione da RTK)

**OUTPUT:**
- Un mapping dettagliato di cosa è stato fatto, cosa manca, quali sono le issue pendenti e come risolverle
