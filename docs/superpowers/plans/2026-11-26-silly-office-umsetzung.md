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
| Rückgabe betrifft eine belegte Kabine und zugleich den Haushalt eines Fahrgasts | Ausziehende Haushalte und Arbeitsplatzverluste werden getrennt gezählt. Jeder verbleibende Fahrgast hat genau einen gültigen Ort. | 10 |
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

### 6. Den ersten Lift freischalten, bauen und benutzen

- [ ] In `crates/sim-core/tests/lift.rs` zunächst mit `cargo test -p sim-core --test lift` die Fälle für 15 zulässige Halte, einen 16. Halt, eine Lücke und einen gesperrten Lift rot ausführen. Bei zwei Aufträgen für denselben Schacht muss der zweite scheitern. Belegung, Saldo und Wegenetz entsprechen anschließend genau einem Bau.
- [ ] In `crates/sim-core/src/lift.rs` die Aktionen zum Freischalten und Bauen über den bestehenden Befehlsweg implementieren. Bauvorschau liefert reale Halte und den ersten blockierten Halt. In `build_menu.gd` die Vorschau auch vor Anmietung als Auskunft zeigen. Beim Bau müssen Freischaltung, Geld und gemietete Fläche ausreichen.
- [ ] Fahrwünsche in Fahrtrichtung bedienen, am letzten angeforderten Halt wenden. Die Wegsuche berücksichtigt Gehzeit, Umstieg sowie geschätzte Warte- und Fahrzeit. Nach Topologieänderungen vor Betreten der nächsten Kante ihre Gültigkeit prüfen. Während einer Fahrt verbleibt die Person bis zu einem gültigen Ausstieg in der Kabine.
- [ ] Im Liftszenario denselben Weg zweimal antreten. Vor dem Bau wird die Treppe benutzt, danach der Lift, wenn seine Gesamtkosten geringer sind. Die Freischaltung bleibt nach Absinken und erneutem Erreichen der Bevölkerungsmarke erhalten, die Marke erzeugt kein zweites Guthaben. Test grün ausführen und den bedienbaren Lift committen.

### 7. Umstieg, Gedränge und Auskunft

- [ ] In `crates/sim-core/tests/interchange.rs` denselben endlichen Ankunftsschub in einen schmalen und einen breiten Vorraum schicken. Bei vollem Vorraum und voller Kabine muss der erste mögliche Ausstieg vor dem nächsten Einstieg erfolgen. Bei gleicher Ankunftszeit entscheidet die Personen-ID. Im breiten Fall sinkt die mittlere Wartezeit der identischen Fahrgastgruppe. Beide Schlangen leeren sich bis zum festgelegten Szenarioende.
- [ ] `cargo test -p sim-core --test interchange` rot ausführen. In `crates/sim-core/src/interchange.rs` gemeinsame Standplätze vor benachbarten Türen, Gangdurchsatz und eine Ausweichregel abbilden. Eine Warteschlange lässt den Ausstiegskorridor frei bzw. rückt zurück. Plätze pro Tick höchstens einmal vergeben. Eine geänderte Flächenwidmung für den Gang muss im selben Modell dessen nutzbare Breite verändern.
- [ ] `World::explain(&self, person: PersonId) -> Option<PersonInfo>` in `crates/sim-core/src/inspect.rs` liefert Ort, Ziel, Grund, betroffene Tür und gemessene Wartezeit. Die Schätzung für neue Wege in festen Zeitfenstern erneuern. Bereits Wartende wechseln bei einem kleinen Kostenvorteil unter der konfigurierten Wechselschwelle ihre Schlange nicht.
- [ ] Den Spielstandtest um einen Umstieg mit Wartenden und besetzter Kabine erweitern. Reihenfolge, Standplätze, Türphase, Fahrplan und nächste Entscheidung müssen nach dem Laden übereinstimmen. Beide Rust-Testziele `--test interchange --test save_resume` grün ausführen, Auskunft und Vorraum im Client anzeigen und committen.

### 8. Den Berufsverkehr bei 100 Stockwerken messen

- [ ] In `crates/sim-cli/tests/benchmark.rs` eine kurze deterministische Prüfung für das Lastszenario anlegen. Genau 100 Geschosse und 5.000 Personen erzeugen. Expresshalte liegen auf Etagen mit durch zehn teilbarer Nummer. Für eine festgelegte Pendlergruppe mindestens einen tatsächlichen Wechsel zwischen Zubringer und Express nachweisen. Zwei kurze Läufe haben dieselben fachlichen Zähler und Kontrollsummen.
- [ ] `cargo test -p sim-cli --test benchmark` rot ausführen. In `crates/sim-cli/src/benchmark.rs` Szenariogenerator, Uhr für das Normaltempo und getrennte Messung der Rechenzeit ergänzen. `godot/content/berufsverkehr.json` legt Anzahl, Linien, feste Saat und eine konzentrierte Ankunftswelle fest. Der Generator schreibt auch das vollständig erzeugte Haus für den Clientvergleich.
- [ ] Nach grünem Funktionstest auf dem protokollierten Referenzgerät `cargo run --locked --release -p sim-cli -- bench godot/content/berufsverkehr.json --seconds 900 --out build/last.json` ausführen. Mindestens 15 reale Minuten mit Anlauf, Stoßzeit und Ruhephase messen. Je Phase Anzahl berechneter Ticks, Tickzeit p95 und Maximum, Wegsuchen, längste Schlange und Spitzenspeicher ausweisen. Zeit für das Warten auf den nächsten Tick zählt nur zur Laufdauer.
- [ ] Gerät, Auflösung, Betriebssystem, Build, Saat und Messwerte in `docs/abnahme/berufsverkehr.md` festhalten. Liegt p95 über 8 ms, den teuersten Anteil profilieren, gezielt ändern und identisch nachmessen. Bei geteilten Routenergebnissen müssen Topologieversion, zulässige Wege und Kostenzeitfenster zusammenpassen. Eine Optimierung mit abweichender Kernprüfsumme erfordert eine fachliche Erklärung. Lastszenario und ersten Befund committen.

### 9. Den Kaffeegang und die Rutschstange entstehen lassen

- [ ] In `crates/sim-core/tests/coffee.rs` denselben Bewohner wiederholt vom Büro zum Kiosk schicken. Vor der im Testszenario festgelegten Wiederholungszahl existiert keine Stange. Danach entsteht genau eine zulässige Abwärtsverbindung. Der nächste Hinweg ist kürzer, der Rückweg führt weiterhin über eine zulässige Aufwärtsverbindung.
- [ ] `cargo test -p sim-core --test coffee` rot ausführen. In `crates/sim-core/src/initiative.rs` Bedarf, wiederkehrenden Umweg, zulässige Anker und Anlass speichern. Einen beobachtbaren Bauzustand mit Fertigstellungstick melden. Erst bei Fertigstellung die gerichtete Kante freigeben. Anlass und beteiligte Personen sind im Client abfragbar.
- [ ] Kurzen Ärger abklingen lassen und wiederholte Störungen als Unzufriedenheit mit messbarem Anlass festhalten. Beobachtungen, angefangene Arbeiten und neue Kante in den Spielstand aufnehmen. Auf halbem Bauweg speichern und laden, anschließend genau eine Stange und denselben Zielweg prüfen. Tests grün ausführen und die erste Bewohnerverbindung committen.

### 10. Einen bewohnten Flügel in einem Tick zurückgeben

- [ ] In `crates/sim-core/tests/return_section.rs` für die folgenden Fälle Vorschau, Bestätigung und Zustand nach dem nächsten Tick prüfen.

  | Ausgangslage | Fachliche Erwartung |
  | --- | --- |
  | Nur der Arbeitsplatz entfällt | Haushalt und Wohnung bleiben. Die Person verliert den alten Arbeitsweg. Ein erreichbarer freier Ersatzplatz wird angenommen. Ohne Ersatz entsteht bis zu einem neuen Angebot kein Arbeitsweg. |
  | Die Wohnung eines mehrköpfigen Haushalts entfällt | Die Vorschau zählt den Haushalt einmal. Alle Mitglieder ziehen aus. Ihre Arbeitsplätze, Kabinen- und Schlangenplätze werden frei. |
  | Einfacher Lift mit Halten 0–4, Rückgabe auf Etage 2 | Halte 0 und 1 bleiben aktiv. 2–4 sind außer Betrieb. Fahrgäste im entfallenden Teil werden umgesetzt. Kein späterer Fahrbefehl bedient den abgetrennten Teil. |
  | Derselbe Lift, Rückgabe auf 0 oder 1 | Der ursprüngliche untere Halt fehlt oder es bleibt nur ein Halt. In beiden Fällen endet der gesamte Betrieb. |
  | Im betroffenen Lift fahren ein ausziehendes Haushaltsmitglied und ein verbleibender Pendler | Das Mitglied verschwindet einmal, der Pendler bekommt genau einen Ort am nächsten weiter zugänglichen Anschluss seiner bisherigen Strecke, ersatzweise an der Rezeption. Alte Ziele und Reservierungen sind aufgelöst. |
  | Im Flügel endet eine Rutschstange oder ein gerade begangener Weg | Die betroffene Verbindung entfällt, die Person erreicht einen gültigen Anschluss. Die Fläche lässt sich im ursprünglichen freien Zustand erneut mieten. |

- [ ] `cargo test -p sim-core --test return_section` rot ausführen. In `crates/sim-core/src/return_section.rs` aus Abschnitt und aktuellem Zustand einen einzigen Änderungsplan ableiten. `preview` liest dessen `Impact` mit Mieteffekt, Auszügen, Arbeitsplatzverlusten, Wegen, Halten und Fahrgästen. Bei Bestätigung am Ausführungstick neu berechnen. Die Bestätigung trägt einen Vergleichswert über die angezeigten Folgen. Haben sich diese geändert, zeigt der Client die aktuelle Vorschau zur erneuten Bestätigung.
- [ ] Bei bestätigtem Ergebnis alle Verträge, Standplätze, Fahrwünsche, Wege, Termine und Verbindungen gemeinsam aktualisieren. Eine stillgelegte Kabine entfernen, eine verbleibende Kabine gegebenenfalls am obersten verbleibenden Halt des unteren Teilstücks wieder einsetzen. Erneutes Anmieten erfordert erneutes Erschließen. Die Rezeption bleibt verfügbar.
- [ ] Mit unverändertem Zustand muss die Vorschau den anschließend ausgeführten Folgen entsprechen. Ändert sich zwischen Vorschau und Befehl ein betroffener Haushalt, bleiben bei Ablehnung sämtliche Ressourcen erhalten. Einen ganzen Spieltag mit Einzug, Liftbau, Kaffee und Rückgabe als `godot/content/spieltag.json` festhalten und durch CLI und Export vergleichen. Paket bei grünem Fachtest und identischen Kontrollpunkten committen.

### 11. Den Gebäudeschnitt spielbar und lesbar machen

- [ ] In `godot/tests/selection.gd` eine Person anklicken, aus dem Bild schwenken, ihre Figurenansicht für jemand anderen verwenden und zurückschwenken. Die Auswahl folgt der ursprünglichen ID. Nach deren Auszug schließt sich die Auskunft oder meldet den Auszug. `--headless -- --test selection` muss die Verwechslung mit einer wiederverwendeten Ansicht erkennen.
- [ ] In `godot/scripts/building_view.gd`, `person_panel.gd` und `godot/art/haus.tres` Etagen, Türen, Kabinen, Personen und Requisiten getrennt zeichnen. Pixelraster, gemeinsame Fuß- und Türanker, Sprite-Atlanten und Importfilter an einer tatsächlichen Figur neben der Kabine abstimmen. Sichtbereich mit Rand gesammelt abfragen, Ansichten wiederverwenden. Nahansicht zeigt Gesten und Becher, Übersicht erhält lesbare Warteschlangen.
- [ ] In `main.tscn` die Eingabeaktionen für Auswahl, Schwenken, Zoomen, Pause, Bau, Rückgabe, Speichern und Laden verbinden. Die Rückgabeansicht zeigt Mietänderung, betroffene Haushalte, verlorene Arbeitsplätze, Halte und Fahrgäste vor dem Bestätigen. Texte für Wartezeiten verwenden die Werte aus `PersonInfo`.
- [ ] Auswahltest grün ausführen. Im exportierten kleinen Haus vom Einzug bis zur Rückgabe spielen. Eine Person bis zur Bürotür verfolgen, nach Liftbau ihren neuen Weg erkennen, Stau am Vorraum durch Verbreiterung lindern und den Bau der Stange beobachten. Übersicht und Nahansicht mit der Stilreferenz vergleichen. Zwei gleichzeitig verständliche Handlungen ohne Eingabe mit Bild und Spieltick in `docs/abnahme/probespiel.md` festhalten. Die geprüfte Darstellung committen.

### 12. Beide Desktop-Builds abnehmen

- [ ] In `godot/tests/desktop_smoke.gd` den vollständigen Lauf als Testfall anlegen. Je Export aus einem frischen Verzeichnis starten, kleines Haus laden, Bauauftrag ausführen, mitten im Umsteigen speichern, Prozess neu starten, laden und bis zum letzten Kontrolltick spielen. Ein fehlendes mitgeliefertes Haus, ein abgelehnter Pflichtbefehl oder eine abweichende Prüfsumme ergibt Exitcode ungleich 0.
- [ ] `.github/workflows/desktop.yml` um diesen Test gegen das ausgelieferte Programm auf Windows und Linux erweitern. Prüfsummen für `spieltag.json` mit dem CLI-Protokoll vergleichen. Ein separater Sichtlauf auf dem Referenz-PC öffnet denselben Export mit Grafik und dem großen Haus aus Paket 8.
- [ ] Bei 1080p über den vollen Umsteigebereich schwenken und aus der Übersicht bis zur Tür zoomen. Bildzeiten p95 und p99 sowie die Zeiten für Kern, Adapter und Darstellung getrennt erfassen. Zum Ziel von 60 Bildern/s den Anteil der Bilder über 16,7 ms angeben. Spielstandsgröße und Ladezeit des großen Hauses protokollieren.
- [ ] Exportjobs und Kernjob müssen grün sein. In `docs/abnahme/probespiel.md` Build, Messgerät, Ergebnisse und Abweichungen zu den Budgets vermerken. Erst nach bestandenem Probespiel und beiden erfolgreichen Neustarts die Artefakte als ersten spielbaren Desktop-Stand kennzeichnen und die Abnahme committen.

**Spätere Plattformentscheidung.** Für einen Mobilprototyp sind zuerst native Bibliothek, Touch-Treffflächen, Speicherbudget und Speichern beim Wechsel in den Hintergrund auf einem echten Gerät zu prüfen. Für den Browser ist der erste Nachweis der gleiche Zwei-Etagen-Lauf mit WebAssembly, geeignetem Renderer und dauerhaftem Laden nach einem Neustart. Beide können denselben Kern und dieselbe Befehlsfolge verwenden.
