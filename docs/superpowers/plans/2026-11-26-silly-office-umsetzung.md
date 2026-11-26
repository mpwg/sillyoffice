# Silly Office – Umsetzungsplan für das erste spielbare Hochhaus

> Für die Umsetzung sind die Schritte als Checkliste angelegt. Bei Bedarf hilft `delegate-coding-efficiently` bei abgegrenzten Programmieraufgaben, `delegate-work-efficiently` bei anderen Arbeitspaketen. Für ausdrücklich direkt ausgeführte Implementierung ist `executing-plans` vorgesehen.

**Ziel.** In einem kleinen PC-Spiel kann man dieselbe Person vom Einzug bis ins Büro verfolgen, ihren Weg durch einen Lift verändern und die Folgen einer Flügelrückgabe beobachten. Ein zweites Haus prüft dieselben Regeln im Berufsverkehr auf 100 Stockwerken.

**Architektur.** Der Rust-Kern besitzt die gesamte Partie. Das Szenarioprogramm und der Godot-Client sind gleichberechtigte Aufrufer. Eine GDExtension übersetzt zwischen Rust und Godot. Ein kompletter Spieltag ist auf einem Rechner ohne installierte Engine reproduzierbar.

**Technik.** Cargo-Workspace, Rust, versioniertes JSON, Godot 4, GDScript und godot-rust. Auf dem Entwicklungsrechner sind Godot 4.6 stable und Rust/Cargo 1.100.0 vorhanden. Für den ersten Export diese Kombination mit passenden Exportvorlagen und kompatibler Binding-Version erproben und die erfolgreichen Versionen im Repository fixieren. [Godot-Rust-Anbindung](https://godot-rust.github.io/book/intro/hello-world.html) · [Godot-Kommandozeile](https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html)

**Grundlage.** [Technischer Bauplan und freigegebene Rückgaberegeln](../specs/2026-11-26-silly-office-technik-design.md) · [Spielkonzept](../specs/2026-11-25-silly-office-design.md) · [Verbindliche Stilreferenz](../../../konzepte/finale-stilreferenz.png)

## Verbindliche Grenzen

- Windows und Linux sind die ersten Exportziele. Maus und Tastatur bedienen dieselben Spielaktionen, an die später Touch gebunden werden kann. Mobil- und Browserexporte gehören zu einem späteren Auslieferungsentscheid.
- Die Kamera darf keinen Spielzustand ändern. Zeit, Geld und Wegkosten sind ganzzahlig. Stabile IDs und feste Reihenfolgen erlauben den Vergleich von Durchläufen derselben Kernversion.
- Der einfache Lift bedient höchstens 15 zusammenhängende Geschosse. Ein Expresslift für das Lastszenario hält nur auf jedem zehnten Geschoss.
- Bei Verlust nur des Arbeitsplatzes bleibt der Haushalt wohnen. Bei Verlust der Wohnung zieht der Haushalt gemeinsam aus. Bei unterbrochenem einfachem Schacht bleibt die zusammenhängende Strecke ab dem ursprünglichen unteren Endhalt mit mindestens zwei Halten aktiv.
- Arbeitswerte für den ersten Versuch sind zehn Ticks pro realer Sekunde bei Normaltempo, 5.000 Personen, höchstens 8 ms pro Tick im 95. Perzentil und 60 Bilder/s bei 1080p. Jede Leistungsaussage braucht ein benanntes Referenzgerät und eine Release-Messung.

## Worauf beim Prüfen besonders zu achten ist

| Fall | Erwartung | Paket |
| --- | --- | --- |
| Zwei gültig vorgeprüfte Bauaufträge konkurrieren um denselben Platz oder das letzte Geld | Der zweite erhält einen konkreten Ablehnungsgrund. Die Kasse wird nur einmal belastet. | 2, 6 |
| Volle Kabine öffnet an einem bereits zugestellten Umstieg | Aussteigende kommen weiter, die Reihenfolge der Wartenden bleibt nachvollziehbar, endliche Nachfrage läuft aus. | 7 |
| Laden mit offener Fahrt, gespeichertem Zukunftsbefehl oder beschädigter Datei | Fortsetzung ergibt denselben Verlauf. Ein Ladefehler lässt die aktive Partie und eine gültige Dateigeneration nutzbar. | 5, 7 |
| Rückgabe betrifft eine belegte Kabine und zugleich den Haushalt eines Fahrgasts | Betroffene werden einmal gezählt. Jeder verbleibende Fahrgast hat genau einen gültigen Ort. | 10 |
| Wiederverwendete Figurenansicht nach Wegschwenken oder Auszug | Auswahl und Auskunft zeigen weiterhin die ursprüngliche ID oder melden deren Auszug. | 3, 11 |

## Dateien und Schnittstellen

Alle folgenden Pfade sind relativ zur Repository-Wurzel. Die Dateien in den Paketen sind geplante Liefergegenstände. Bisher enthält das Repository Spielkonzept, technischen Bauplan und Gestaltungsvorlagen.

| Ort | Inhalt |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Rust-Pakete und reproduzierbare Werkzeugauswahl |
| `crates/sim-core/src/` | Szenariodaten, Spielzustand, Befehle, Zeit, Wege, Lifte, Wirtschaft, Auskünfte und Spielstände |
| `crates/sim-cli/src/main.rs` | Liest Dateien, führt Befehlsfolgen aus, schreibt Kontrollpunkte oder Messbericht |
| `crates/sim-godot/src/lib.rs` | Eine Kerninstanz je Partie, Übersetzung der Ein- und Ausgaben |
| `godot/content/` | Ein gemeinsamer Satz gewöhnlicher JSON-Dateien für CLI und Export. Der Kern bekommt die Bytes vom Aufrufer. |
| `godot/project.godot`, `godot/main.tscn`, `godot/scripts/`, `godot/art/` | Bedienbare PC-Partie, Weltdarstellung und Zuordnung der Grafik |
| `crates/sim-core/tests/`, `crates/sim-cli/tests/`, `godot/tests/` | Fachtests, Test des Dateiaufrufers und Abnahme durch den Client |
| `.github/workflows/`, `docs/abnahme/` | Builds ohne Engine, native Exporte und reproduzierbare Messprotokolle |

Der Kern bietet `World::new(scenario: Scenario, seed: u64) -> Result<World, DataError>`, `submit(&mut self, command: ScheduledCommand) -> Result<(), SubmitError>` und `step(&mut self) -> TickReport`. Ein Auftrag enthält Ziel-Tick, eindeutige Nummer und Aktion. Annahme bestätigt die Einreihung. Das fachliche Ergebnis entsteht im Ziel-Tick. Pro Tick zuerst Befehle in Nummernfolge, dann fällige Termine in stabiler Reihenfolge abarbeiten. Lesende Abfragen liefern Daten mit dem letzten abgeschlossenen Tick. Ein Tick darf vollständig laufen, auch wenn zwischen zwei Bildern mehrere Ticks fällig sind.

## Arbeitspakete

Die Pakete sind in Ausführungsreihenfolge angeordnet. Die in Testbeispielen verwendeten Werte gehören zur jeweiligen Testpartie. Je Paket zuerst die angegebene Prüfung mit rotem Ergebnis ausführen, dann implementieren, dieselbe Prüfung grün ausführen und den beschriebenen Stand committen. Bei den Desktop-Paketen gehört der Start des exportierten Programms zum Test.

### 1. Zwei Stockwerke aus einer Datei laden

- [ ] In `crates/sim-core/tests/scenario.rs` den Test `loads_two_floors` anlegen. Der Grundriss hat zwei Etagen, Rezeption, Büro, eine Treppe, einen freien Seitenflügel und eine bereits eingezogene Person. Für doppelte IDs, ungültige Verweise, Halte außerhalb des Rasters und unbekannte Formatversion jeweils den fehlerhaften Eintrag prüfen.
- [ ] `cargo test -p sim-core --test scenario` zunächst rot ausführen. Workspace und Paketmanifest mit diesem Test anlegen.
- [ ] In `crates/sim-core/src/scenario.rs` den Eingang `Scenario::from_json(bytes: &[u8]) -> Result<Scenario, DataError>` implementieren. Statische Geometrie von der Anfangsbelegung trennen. Das Referenzhaus in `godot/content/zwei-etagen.json` ablegen. `lib.rs` exportiert das Lademodell.
- [ ] Den Test grün ausführen und den validierten Grundriss samt fixierter Rust-Werkzeugkette committen.

### 2. Den ersten Arbeitsweg ohne Engine nachspielen

- [ ] `crates/sim-cli/tests/replay.rs` prüft `first_walk_is_repeatable`. Starttick 0, Saatwert 42, Anmietung bei Tick 5, Ankunft im Büro bei Tick 12. Bei Tick 4 ist der Flügel frei, bei Tick 5 gemietet und bei Tick 12 befindet sich die Person im Büro. Ein zweiter identischer Mietversuch bei Tick 5 wird abgelehnt. Zwei Läufe liefern identische Kontrollpunkte.
- [ ] Mit `cargo test -p sim-cli --test replay` das fehlende Verhalten nachweisen. In `crates/sim-core/src/world.rs` und `command.rs` den oben definierten Vertrag umsetzen. Termine und Gehsegmente tragen Start- und Endtick. Vergangene Ziel-Ticks und doppelte Befehlsnummern bereits bei der Einreihung ablehnen.
- [ ] `World::checkpoint(&self) -> Checkpoint` enthält Tick, Befehlsergebnisse des Ticks und eine stabile Prüfsumme über kanonisch geordnete, zukunftsrelevante Fachdaten einschließlich wartender Befehle und Zufallszustand. In der CLI `replay HAUS ABLAUF --out DATEI` implementieren. Jeder Aufrufer liest Kontrollticks und Saatwert aus `godot/content/erster-weg.json`.
- [ ] In `.github/workflows/core.yml` auf einem Runner ohne Godot `cargo test --locked -p sim-core -p sim-cli` ausführen. Lokal und in diesem Job müssen die fachlichen Erwartungen und beide Durchläufe übereinstimmen. CLI und Kernlauf gemeinsam committen.

### 3. Den ersten Arbeitsweg im exportierten Spiel wiederholen

- [ ] In `godot/tests/replay.gd` denselben Ablauf über den normalen Partiezustand ausführen. Abwechselnd Büro und Rezeption betrachten. Die Datei der CLI ist die Referenz für alle drei Kontrollticks und Befehlsergebnisse. Der Exporttest muss zunächst an der fehlenden Anbindung scheitern.
- [ ] `crates/sim-godot/src/lib.rs` als `cdylib` mit einer Godot-Klasse `SimSession` aufbauen. Diese kapselt Laden, Einreihen, Fortschalten und Kontrollpunkte. `World::view(&self, area: ViewRect) -> ViewFrame` liefert nur die angeforderten Etagen, stabile IDs und Bewegungssegmente gesammelt. `godot/scripts/session.gd` liest Kernzeit und interpoliert die sichtbare Person. Pause hält die Spielzeit beim Schwenken fest.
- [ ] In `godot/project.godot`, `main.tscn`, `sim.gdextension` und `export_presets.cfg` unterhalb von `godot/` die Startszene, plattformspezifische native Bibliotheken unter `res://bin/` und den Export der JSON-Daten einrichten. In `.github/workflows/desktop.yml` Editor, Vorlagen und Rust-Binding passend fixieren. Je ein nativer Linux- und Windows-Job baut und kopiert die Bibliothek, importiert das Projekt und exportiert es.
- [ ] Auf Linux etwa `godot --headless --path godot --export-release Linux ../build/SillyOffice` nach Anlegen von `build/` ausführen. `build/SillyOffice --headless -- --replay erster-weg --out godot-run.json` muss wie das Windows-Pendant mit Exitcode 0 enden. Der Test vergleicht die geladenen JSON-Einträge mit dem CLI-Lauf, auch bei abweichender Kamerafahrt. Beim ersten Unterschied Tick und beide Zustände ausgeben. Den funktionierenden Durchstich committen.

### 4. Vom leeren Haus bis zur Bürotür spielen

- [ ] `crates/sim-core/tests/first_day.rs` startet `godot/content/erstes-haus.json` mit Interessenten an der Rezeption. Erst Wohnraum, dann ein unerreichbares Büro widmen. Es zieht niemand ein. Ein erreichbares Büro ermöglicht genau einen Einzug. Die erste Person behält auf Treppe und im Büro ihre ID. Am Abrechnungstick gilt `Endsaldo = Anfangssaldo + belegte Mieten − Abschnittsmieten − Einmalkosten`.
- [ ] `cargo test -p sim-core --test first_day` rot ausführen. In `tenancy.rs`, `economy.rs` und `routing.rs` in `crates/sim-core/src/` Haushalte, Verträge, Rezeption, Ganzzahlbuchungen und gerichtete Wegsuche mit Dijkstra ergänzen. Gleich teure Wege in stabiler ID-Reihenfolge auflösen. Erste Bevölkerungsmarke gewährt die konfigurierten Punkte einmal pro Partie.
- [ ] Im Kern `World::preview(&self, action: &Action) -> Result<Impact, RuleError>` bereitstellen. In `godot/scripts/build_menu.gd` Anmieten und Widmen über diese Vorschau und dieselben Regeln am Ausführungstick anbinden. In der kleinen Karte Wohnung, Büro, späteren Kiosk, zulässigen Schacht und einen separat mietbaren Seitenflügel vorsehen.
- [ ] Test grün ausführen. In der exportierten Partie die Widmungen per Maus setzen, die Kosten prüfen und die Person ins Büro verfolgen. Den spielbaren Einzug committen.

### 5. Unterwegs speichern und dieselbe Partie fortsetzen

- [ ] In `crates/sim-core/tests/save_resume.rs` den Test `save_mid_walk` zuerst rot ausführen. Während des Gehens mit einem eingereihten Zukunftsbefehl speichern. Nach dem Laden jeden Kontrollpunkt bis zur Ankunft und die nächste Zufallsentscheidung gegen den durchgehenden Lauf vergleichen.
- [ ] In `crates/sim-core/src/save.rs` die Schnittstellen `World::save(&self) -> Result<Vec<u8>, SaveError>` und `World::restore(scenario: Scenario, bytes: &[u8]) -> Result<World, SaveError>` implementieren. Szenario- und Regelversion prüfen. Termine, nächste IDs, Befehle, Zufallszustand und Fortschritt müssen den nächsten Tick vollständig bestimmen. Indizes neu aufbauen.
- [ ] In `godot/scripts/save_store.gd` in eine temporäre Datei im selben Verzeichnis schreiben, die Vollständigkeit prüfen und eine gültige Vorgängergeneration behalten. Einen geladenen Kern erst nach vollständiger Prüfung einsetzen. `godot/tests/save_store.gd` unterbricht jeden Austauschschritt und versucht den Neustart. Mindestens eine gültige Generation muss lesbar bleiben. Manuelles Laden einer unbekannten Version oder einer ungültigen ID meldet den Grund und erhält die aktive Partie.
- [ ] `cargo test -p sim-core --test save_resume` und den Exporttest `--headless -- --test save-store` auf beiden Systemen grün ausführen. Dateigenerationen und Wiederaufnahme gemeinsam committen.
