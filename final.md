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







Sei un esperto sviluppatore di kernel linux e comandi nativi di basso livello, con forti competenze in RUST, memory management e software critici ad altissime prestazione.
Stai realizzando una suite di comandi per Linux / MacOSX e Windows. 
Tutto ciò che scrivi o implementi deve rispettare la lingua inglese (manuali, commenti, localizzazioni, etc.).
Di seguito ti riporto un'analisi su possibili nuovi comandi da implementare.

**OBIETTIVO**
Analizzare a fondo il mercato dei comandi esistenti per verificare che effettivamente siano non già esistenti e che abbiano spazio di "mercato" e di utilizzo oltre che di supporto effettivo per agent-coding.

**OUTPUT** 
Una valutazione dettagliata per ogni comando con valutazione di:
- valutazione validità di mercato
- copertura e impatto
- da realizzare SI/NO

Inoltre realizzerei sotto la cartella docs/evolutive un documento per ogni comando in formato .md che sia adatto per codex/claude code per poterli implementare come ulteriori comandi dell'attuale suite e quindi seguendo la stessa logica generale:
- trovare un nome axt-<comando> che non sia già preso
- abilitare un alias opzionale
- avere man pages e skill e aggiornare lo skill-installer
- avere tutti i test necessari
- essere cross-platform più possibile
- nativo rust single-binary
- ultra-performante

**Questa l'analisi su cui lavorare**
Ho passato un po' di tempo a scavare in issue di Claude Code, post di chi gira agenti in produzione, dati di telemetria OTel e progetti tipo RTK/Serena/ast-grep/Probe. Sotto c'è prima cosa risulta dai dati su quali pattern di comandi gli agenti usano davvero, poi dieci proposte concrete di CLI da scrivere in Rust che riempono buchi reali — non "ennesimo grep più veloce".

## Cosa dicono i log e la telemetria

I bash command che gli agenti chiamano più spesso e che sono noti come problematici sono pochi e ricorrenti. Una issue ufficiale di Claude Code li elenca con dati: tra i casi più frequenti ci sono `sed -n` per leggere range di righe (≈45 occorrenze, ~40 problematiche), `grep -r/-c/-A/-B/-C` per ricerche ricorsive o con contesto, e `cat > file <<EOF` per scrivere file via heredoc, spesso in pipeline con `head` o concatenati con `&&`. Anche dopo che il system prompt vieta esplicitamente questi comandi, il modello continua a usare `cat`, `grep`, `head`, `tail`, `sed`, `find` in circa il 40% delle sessioni più lunghe di 30 minuti, soprattutto dopo compaction o nei sub-agent.

Sul fronte dei costi: RTK misura che, in una sessione tipica TS/Rust di 30 minuti, l'output di shell occupa ~118K token che si possono comprimere a ~24K — circa 88K token "buttati" via comandi tipo `git status`, `cargo test`, `ls -la`. Anthropic stessa, nel paper "Code execution with MCP", ha mostrato un workflow che passava da 150.000 a 2.000 token (–98,7%) semplicemente eseguendo il lavoro "fuori" dal contesto del modello e restituendo solo il risultato. Il principio è chiaro: **ogni byte non strettamente necessario alla decisione del modello è uno spreco**.

Il pattern dominante che ho visto ripetersi nei log è il loop *search → read → search → read*: l'agente fa un grep, legge 200 righe per capire il contesto, fa un altro grep, legge un altro file, e così via. Ogni passo costa un round-trip al modello. È esattamente lì che ci sono margini grossi.

## Dieci CLI Rust che riempirebbero buchi reali

### 1. `ctxpack` — multi-pattern, multi-file, output strutturato in una chiamata

Il caso che hai citato tu. Oggi l'agente fa N grep separati ("cerca `TODO`", poi "cerca `FIXME`", poi "cerca `unwrap()`"); ognuno è un tool call con il suo overhead. Idea: `ctxpack --files 'src/**/*.rs' --pattern todo=TODO --pattern fixme=FIXME --pattern panics='unwrap\(|expect\(' --context 2 --json`. Output JSON: per ogni pattern, lista di hit con `file`, `line`, `kind` (è dentro una funzione? un commento? un test?), snippet di N righe. Una chiamata, output denso, semantica già preparata. Va oltre `rg --json` perché correla pattern multipli e li classifica per tipo di nodo AST tramite tree-sitter — qualcosa di simile a quello che Probe fa già combinando ripgrep con AST tree-sitter per "raccogliere il contesto completo in un'unica chiamata", risparmiando i 10+ giri agentici tipici.

### 2. `outline` — solo le firme, niente corpo

L'agente apre `mod.rs` da 2000 righe per capire "cosa esporta questo modulo". Spreco enorme. `outline path/` restituisce un JSON di soli simboli pubblici: nome, tipo (`fn`/`struct`/`impl`/`trait`/`const`), firma, doc-comment, range. RTK ha una `read --aggressive` che strippa i corpi e tiene solo le signature, e questo è esattamente il livello di dettaglio giusto per la navigazione di codebase. Un tool dedicato che lo fa multilingua via tree-sitter (Rust, TS, Python, Go, Java) sarebbe oro: in genere –80–90% di token rispetto a un `cat`.

### 3. `slice` — leggere per simbolo, non per riga

Il problema #1 dei modelli con `sed -n '253,343p'` è che tradurre "around line 150" in numeri di riga richiede un passaggio mentale che spesso sbagliano, e si perde quando il file viene editato. `slice file.rs --symbol fn::process_request` ti restituisce esattamente quella funzione (con docstring e attributi sopra). `slice --before-symbol foo --include-imports` ti aggiunge gli use rilevanti. Niente più line range fragili. Lavora con LSP o tree-sitter come fallback. Risolve la causa radice della "riga 250 non più valida dopo l'edit".

### 4. `repomap` — tutta la topologia in una shot

All'inizio di ogni sessione l'agente fa: `ls`, `cat README`, `cat package.json` (o Cargo.toml/pyproject), `find . -name 'test_*'`, `git log --oneline -10`. Cinque tool call, output rumoroso, sempre uguale. `repomap` restituisce un JSON unico con: layout (workspace members, src/tests/examples), build system rilevato, entry point, manifest delle dipendenze normalizzato, lingua principale, framework di test, recent commits compatti. Una sola chiamata e l'agente sa di che progetto sta parlando. Il principio guida del post "AI Can't Read Your Docs" di Shrivu Shankar è che l'output di un tool deve essere un buon prompt per il turno successivo, non un dead-end: bisogna restituire info dense e già strutturate, non un cat che obbliga a un altro giro.

### 5. `testdigest` — solo i fallimenti, formato uniforme cross-runner

`cargo test`, `pytest`, `jest`, `go test` sputano centinaia di linee dove all'agente serve solo: quali sono falliti, dove (file:line cliccabile), assertion, stderr breve. RTK ha already wrapper specifici per `jest`, `vitest`, `pytest`, `cargo test`, `go test` con riduzione ~90% mostrando solo i failure, ma sono compressioni indipendenti. Un `testdigest` Rust che gira il runner giusto in base al progetto e restituisce un **JSON unificato** (`{failures: [{file, line, name, message, stack[3 frames]}], passed: N, duration_ms}`) sarebbe il go-to per i CI/agenti. Bonus: flag `--rerun-failed-only` perché è la prossima cosa che l'agente fa il 100% delle volte.

### 6. `gitctx` — bundle di stato git in un colpo

Pattern ricorrente: `git status` + `git branch -vv` + `git log --oneline -10` + `git diff --stat` + `git diff` su file modificati. Cinque tool call sequenziali. `gitctx` fa tutto in parallelo, output JSON denso: branch, ahead/behind, lista file con `{path, status, +/-, hunk_count}`, ultimi N commit, e — il pezzo che fa la differenza — i diff veri solo per i file più piccoli (sotto soglia), gli altri solo come stat. Risparmio token tipico misurato in chi compone questi pezzi: ~70%.

### 7. `manifests` — leggere tutti i config di progetto come schema unificato

`package.json`, `tsconfig.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Dockerfile`, `.github/workflows/*.yml`. Ogni progetto ne ha 5–10. L'agente li `cat` uno per uno. `manifests --normalize` restituisce un JSON con schema unificato: `{deps: [...], dev_deps: [...], scripts: {...}, build: {...}, runtime_versions: {...}, ci_jobs: [...]}`. Diventa la base per domande tipo "ho una vulnerabilità in lodash, dove la uso?" senza leggere niente. Nessuno l'ha fatto bene multi-linguaggio.

### 8. `impact` — blast radius di una modifica

Domanda costante: "se cambio `parse_config()`, cosa si rompe?". Oggi l'agente fa `grep parse_config -r .`, poi legge ogni file, poi cerca i test. Serena via LSP riduce questi 8–12 step error-prone in una chiamata atomica usando find-references, e gli agenti la chiedono esplicitamente come il singolo tool più impattante. Ma Serena è MCP. Una CLI Rust che embedda un client LSP minimale (rust-analyzer, pyright, gopls, ts-server) ed espone `impact --symbol foo --file bar.rs` → JSON `{call_sites: [...], tests_touching: [...], suggested_review_files: [...]}` colmerebbe il gap per chi non vuole MCP.

### 9. `plan-edit` — dry-run di un find-and-replace strutturale

`sed -i` è pericoloso: la sostituzione regex può corrompere file silenziosamente, a differenza di un Edit con validazione di unicità e diff chiaro. `plan-edit --pattern '...' --replacement '...' --lang rust` (basato su ast-grep) fa il **piano**: JSON con tutte le occorrenze, contesto, e un diff proposto, **senza scrivere nulla**. L'agente legge il piano, decide, poi `plan-edit --apply <plan-id>` esegue atomicamente con backup. Risolve sia il problema di sicurezza sia quello del "cambiamento parziale" che lascia il repo rotto.

### 10. `logsift` — triage di log/output enormi

Quando l'agente debugga, riceve in input file di log da decine di MB o output di build verbosi. `logsift app.log --since 1h --severity error+ --dedup --top 20` autodetecta il formato (syslog, JSON, log4j, nginx), deduplica linee ripetute con conteggio, estrae stack trace come blocchi unici, e restituisce timeline + top errori. Una guida pratica recente di Claude Code mostra come una pipeline simile (parser universale auto-detect formato, filtro per range temporale, frequenza degli errori, preset per errori/auth/slow request) sia esattamente lo strumento che serve quando "qualcosa è rotto". Niente in Rust con questa specifica forma agent-friendly esiste ancora.

---

## Principi trasversali da tenere fissi

Tre cose che ho visto come vincenti in tutti i progetti del settore, e che dovresti prendere come default fissi:

**Output strutturato JSON come prima classe, non come `--json` opzionale.** L'agente ha bisogno di parsare, non di leggere. Aggiungi `--format text` per gli umani, non viceversa.

**Output che suggerisce il next step.** Un comando che riesce non deve restituire solo un OK silenzioso o un ID: deve essere un prompt per il turno successivo dell'agente, suggerendo i comandi o gli ID più probabili da invocare dopo. Esempio: dopo `testdigest` che mostra 3 fallimenti, includi `next: testdigest --rerun-id <fail-id> --verbose`.

**Errori actionable.** Stessa logica per i fallimenti: spiega cosa è andato storto e come si ripara, non solo "exit 1".

Se devi sceglierne tre da cui partire, andrei su `outline`, `testdigest`, e `ctxpack`: sono i tre dove il guadagno in token e in giri agentici è massimo, il dominio è chiaro, e nessuno li ha ancora fatti bene tutti insieme. `slice` come quarto se vuoi attaccare il problema più strutturale (line numbers fragili).