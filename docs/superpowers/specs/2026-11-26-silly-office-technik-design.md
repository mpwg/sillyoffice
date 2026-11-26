# Silly Office – technischer Entwurf

**Stand 26. November 2026, Entwurf zur Freigabe.** Gewählt sind Godot 4 und der PC als erstes Ziel. Der Nutzer wünscht ein Hochhaus mit ungefähr 100 Stockwerken und einen Simulationskern, der grundsätzlich unabhängig von der Oberfläche läuft. Rust, die konkrete Anbindung, die Beispielbelegung und die Leistungsziele unten sind technische Vorschläge.

## Was der erste Bauabschnitt beweisen soll

Der Maßstab ist der [erste spielbare Ausschnitt im Spielkonzept](2026-11-25-silly-office-design.md). Ein Interessent findet an der Rezeption Wohnung und Arbeit, geht über die Treppe ins Büro und benutzt später den vom Spieler gebauten Lift. Am Kaffeekiosk führt ein wiederkehrender Umweg zu einer von Bewohnern gebauten Rutschstange. Die Rückgabe eines Flügels verändert Haushalt, Mietverhältnisse und Wege. Der Nutzer soll eine Person durch diese Veränderungen hindurch wiedererkennen und ihren Entscheidungen folgen können.

Das kleine Haus prüft, ob das Spiel als Aufbauspiel funktioniert. Ein zweites, synthetisches Haus prüft früh, ob dieselbe Technik 100 Stockwerke tragen kann. Als erste Messgröße schlage ich 5.000 gleichzeitig vorhandene Personen vor. Die Belegung wird als Szenarioparameter geführt. Viele am Umsteigebahnhof eintreffende Pendler sind ein eigener Lastfall, weil eine gleichmäßige Verteilung über den Tag den teuersten Fall verdecken kann.

## Wahl der Plattform

| Betrachteter Ansatz | Nutzen für dieses Spiel | Aufwand und Entscheidung |
| --- | --- | --- |
| Godot 4 als PC-Client | Der 2D-Editor erlaubt kurze Iterationen an Stockwerken, Türen, Gesten, Zoom und Licht. Ein lokales Spiel kann ohne Server laufen. | Vom Nutzer gewählt. Vorgeschlagene erste Auslieferung für Windows und Linux. Beide erhalten denselben Simulationskern. |
| TypeScript mit PixiJS im Browser | Ein Link genügt für Probespiele. Ein Web Worker könnte die Simulation von der Darstellung entkoppeln. | Browser-Lebenszyklus und Austausch mit dem Worker müssten von Beginn an getestet werden. Für Etagen und Animationen wäre ein eigener Autorenworkflow nötig. |

Für die gewählte Variante schlage ich **eine eigenständige Rust-Bibliothek, eine schmale GDExtension und Godot-Szenen mit GDScript** vor. Ein GDScript-Kern ließe sich ebenfalls ohne sichtbares Fenster testen, würde für diese Tests aber die Godot-Laufzeit benötigen. Die Bibliothek gibt uns einen ausführbaren Nachweis der Unabhängigkeit. Sie lässt sich mit einem normalen Rust-Test oder Szenarioprogramm ganz ohne Engine verwenden. Der Preis sind ein zweiter Build und eine native Bibliothek je Zielplattform.

## Grenze zwischen Kern und Oberfläche

| Baustein | Zuständigkeit | Beispiel |
| --- | --- | --- |
| Simulationskern in Rust | Besitzt Spielzeit, Personen, Gebäudetopologie, Liftbetrieb, Entscheidungen, Kasse, Zufallszustand und Spielstandsmodell. Benötigt weder Fenster noch Dateisystemzugriff für einen Simulationsschritt. | Bestimmt, welcher Fahrgast beim Öffnen der Tür aussteigt und weshalb der nächste Fahrgast warten muss. |
| Godot-Anbindung in Rust | Besitzt eine Kerninstanz pro geladenem Spiel. Übersetzt Eingaben, Zeitfortschritt, Abfragen und kompakte Datenpakete an der Engine-Grenze. | Überführt die Liste sichtbarer Personen gesammelt in Godot-kompatible Arrays. |
| Godot-Client | Besitzt Kamera, Auswahl, Eingabegeräte, Menüs, Grafik, Audio und die Ablage der Spielstandsdateien. | Zeigt die wartende Person, spielt die Türanimation und öffnet beim Anklicken ihre Auskunft. |
| Szenarioprogramm ohne Godot | Lädt Testhäuser, gibt dieselben Spielbefehle wie der Client und schreibt Messwerte oder Zustandsprüfsummen. | Lässt den Berufsverkehr desselben Hauses zehnmal mit denselben Eingaben ablaufen. |

**Vertrag.** Der Aufrufer übergibt Befehle wie Abschnitt mieten, Fläche widmen oder Lift bauen. Eine Befehlsnummer und der Ausführungstick legen die Reihenfolge fest. Der Kern prüft Vorbedingungen und liefert Erfolg oder einen strukturierten Ablehnungsgrund. Eine Bauvorschau fragt dieselben Platzierungsregeln lesend ab. Zeit verstreicht nur durch expliziten Aufruf zum Fortschalten. Abfragen liefern unveränderliche Ausschnitte über stabile IDs, etwa die sichtbaren Bewegungssegmente, die Wartenden an einer Tür oder die Ursache für den Ärger einer Person. Der Kern ist die einzige Stelle, die Spielzustand ändert. Das Ende einer Türanimation löst deshalb keinen Einstieg aus. Die Tür öffnet sich für die im Kern festgelegte Zeit. Die Darstellung liest ihren Fortschritt aus dem aktuellen Tick.

**Beispiel.** Der Client fragt für einen Schacht die bedienbaren Halte ab. Nach dem Klick prüft der Kern am Ausführungstick erneut Platz, Freischaltung und Geld. Erst ein erfolgreicher Befehl belastet die Kasse und ändert das Wegenetz. Bereits laufende Wege werden an geeigneten Entscheidungspunkten neu bewertet. Derselbe Vorgang muss sich mit einer Befehlsfolge in einem Test ganz ohne Godot auslösen lassen.

### Der erste ausführbare Nachweis

Für den ersten Bauabschnitt schlage ich drei Rust-Pakete vor. `sim-core` ist eine gewöhnliche Bibliothek mit den Spielregeln. Das Szenarioprogramm `sim-cli` und die native Bibliothek `sim-godot` hängen jeweils direkt von ihr ab. Allein `sim-godot` bindet die Godot-API ein. Ein eigener Build-Schritt kompiliert und testet `sim-core` und `sim-cli` auf einem Rechner ohne Godot. Für den Desktop-Export legt der Build die passende Erweiterungsbibliothek innerhalb des Godot-Projekts ab. Die Zuordnung von Zielplattform und Bibliothek steht in der GDExtension-Konfiguration. [Technische Referenz für den Rust-Adapter und die Exportpfade](https://godot-rust.github.io/book/intro/hello-world.html)

Der erste Vergleich braucht nur zwei Etagen, eine Treppe und eine Person auf dem Weg ins Büro. Szenarioprogramm und exportierter Client laden denselben Grundriss, Startwert und Befehlsverlauf. Zu einem vorgegebenen Tick mieten sie einen freien Seitenflügel an. Beide protokollieren unmittelbar vor dem Befehl, nach seiner Ausführung und bei der Ankunft im Büro den Tick, das Befehlsergebnis (soweit vorhanden) und die Prüfsumme des Kernzustands. Bei einer Abweichung liefert der erste abweichende Kontrollpunkt beide Kernzustände zum Vergleich. Ein zweiter Durchlauf schwenkt die Kamera während des Gehens vom Büro weg. Der Vergleich muss bei unveränderten Eingaben dieselben Ergebnisse liefern.

## Weltmodell und Zeit

- **Gebäude.** Ein handgebauter Grundriss beschreibt Stockwerk, Rasterkoordinate, Abschnitt, mögliche Zugänge, Hindernisse und erlaubte Halte. Der laufende Spielzustand ergänzt Mietverträge, Widmungen, belegte Räume, freigeschaltete Technik und gebaute Verbindungen. Stabile IDs verbinden beide Ebenen. Godot-Node-Pfade und Pixelpositionen gehören zur Darstellung.
- **Menschen.** Eine Person behält Identität, Wohnung, Arbeitsplatz, Tätigkeit, aktuelles Ziel und Reiseabschnitt. Haushalte und Mietverhältnisse erhalten eigene IDs. Eine Person kann etwa bis Dienstschluss arbeiten, zu einer Haltestelle laufen, in ihrer Schlange warten oder in einer Kabine fahren. Diese Zustände bleiben auch außerhalb des Bildes erhalten.
- **Zeit.** Vorschlag für den Einstieg sind zehn feste Simulationsticks pro realer Sekunde bei Normaltempo. Spieluhr und Tageslänge werden als Szenarioregel festgelegt. Lange Tätigkeiten planen ihren nächsten Zustandswechsel voraus. Für eine freie Gangstrecke genügen Anfang, Ziel und Ankunftstick. Für belegte Engstellen berechnet der Kern die tatsächliche Freigabe. Godot interpoliert Positionen zwischen bestätigten Zuständen.
- **Ausführung.** Zunächst hat jede Partie einen sequenziellen Schreiber. Bei normalem Tempo ruft der Adapter den Kern aus der Godot-Hauptschleife auf. Der Vertrag erlaubt später einen eigenen Simulationsthread mit geordnetem Befehlseingang und fertigen Lesepaketen, falls Messungen dies rechtfertigen. Pause stoppt die Spielzeit, die Kamera bleibt bedienbar. Bei zu langsamer Berechnung darf das Spiel langsamer laufen. Spielentscheidende Ticks werden vollständig abgearbeitet.
- **Reproduzierbarkeit.** Für dieselbe Kernversion ergeben Szenario, Zufallsstartwert und geordnete Befehle denselben Zustand. Reihenfolgen bei gleichzeitigen Ereignissen sind fest definiert. Zeit, Geld, Kapazitäten und Wegkosten verwenden ganzzahlige Einheiten. Zustandsprüfsummen beziehen sich auf kanonisch geordnete Fachdaten. Renderfrequenz, Sprache und Sichtbereich sind keine Eingänge in Spielregeln.

## Wege, Aufzüge und sichtbarer Stau

Ein gerichteter Graph verbindet Raumtüren, Gangabschnitte, Treppen, Liftzugänge und Abkürzungen. Die Richtung bildet etwa die nur abwärts nutzbare Rutschstange ab. Wegkosten umfassen Gehzeit, geschätzte Wartezeit, Fahrt und Umstieg. Tatsächliche Wartezeiten ergeben sich aus dem Betrieb. Eine Liftfahrt wird als Einsteigen, Fahrt und Aussteigen geplant. Kabinen, Türen und Warteschlangen bleiben dabei eigene Objekte.

Für den Einstieg genügt Dijkstra auf diesem kompakten Graphen. Eine vorberechnete Suche zu einem häufigen Ziel lässt sich teilen, solange Gebäudenetz, zugelassene Wege und das Zeitfenster der Wartezeitschätzung übereinstimmen. Umbauten ändern die Version, gesperrte Verbindungen werden vor Benutzung erneut geprüft. Wartezeitschätzungen werden in Zeitfenstern erneuert. Wer bereits ansteht, wechselt die Route nur bei einer relevanten Änderung oder einem ausreichend großen Zeitgewinn. So führt eine kurz schwankende Schätzung nicht zu ständigem Schlangenwechsel.

Die erste Kabine bedient registrierte Halte in Fahrtrichtung und wechselt die Richtung nach dem letzten anstehenden Halt. Beim Türöffnen steigen zuerst Fahrgäste aus, anschließend Wartende in passender Richtung bis zur freien Kapazität ein. Die Reihenfolge folgt Ankunftstick und Personen-ID. Vor den Türen besitzen Gangabschnitte maßstabsabhängige Standplätze und Durchsatzgrenzen. Wartende belegen Platz, Aussteigende brauchen Platz im Zielabschnitt. Ein gemeinsam genutzter schmaler Vorraum kann die Tür des Nachbarlifts blockieren. Für den Türbereich definiert der Kern eine Räumregel mit Vorrang für Aussteigende und zurücktretenden Wartenden. Das verhindert einen künstlichen Dauerstillstand. Eine Verbreiterung schafft zusätzliche Plätze und Durchgänge. Godot zeigt die vom Kern zugewiesenen Positionen. Starre Körper und Kollisionen pro Bewohner sind dafür entbehrlich.

Linientypen werden über zulässige Halte, Reichweite, Geschwindigkeit und Kapazität beschrieben. So kann der einfache Lift höchstens 15 Geschosse mit durchgehend möglichen Halten bedienen, ein späterer Expresslift hält nur auf jedem zehnten Geschoss. Der erste spielbare Ausschnitt braucht Treppe, einfachen Lift und Rutschstange. Das große Testhaus darf Express- und Zubringerlinien vorkonfigurieren, um echte Umsteigelast zu erzeugen.

## Mieter, Wirtschaft und nachvollziehbare Entscheidungen

Die Rezeption verwaltet Interessenten. Wohnraum und Arbeitsplatz werden für den ersten Einzug zusammen auf freie Kapazität und Erreichbarkeit geprüft. Einnahmen und Abschnittsmieten bucht der Kern auf der Spieluhr in kleinsten Geldeinheiten. Schwellen für Entwicklungspunkte werden pro Partie einmal verbucht. Bedarf, Unzufriedenheit und Beobachtungen wiederkehrender Umwege werden als kompakte Fachdaten geführt. Ein erklärbarer Grund enthält Typ, betroffene IDs, Zeitpunkt und passende Messwerte. Daraus kann der Client einen Satz wie „Heute Morgen habe ich zwölf Spielminuten am Lift gewartet“ erzeugen.

Beim Kaffeegang prüft der Kern, ob wiederkehrende Umwege eine zulässige Bewohnerverbindung rechtfertigen. Er erzeugt die Rutschstange, hält ihren Anlass fest und aktualisiert den Graphen. Bei einer Abschnittsrückgabe zeigt eine Vorschau Mieteffekt, Auszüge und entfallende Wege. Die bestätigte Rückgabe ändert die angemietete Fläche, Bewohnerverbindungen, Verträge und Wege gemeinsam an einer Tickgrenze. Personen auf einem entfernten Wegstück erhalten im selben Vorgang einen gültigen Anschluss. Vorgeschlagene Regel für unvermeidbare Verdrängung ist der nächste weiter zugängliche Anschluss der bisherigen Strecke, ersatzweise die Rezeption. Ausgezogene Mieter geben ihre Plätze in Warteschlangen und Kabinen frei.

**Zwei vorgeschlagene Rückgaberegeln für das erste Haus.** Die Vorschau und die bestätigte Rückgabe verwenden dieselbe Berechnung.

- Entfällt nur der Arbeitsplatz, bleibt der Haushalt in seiner Wohnung. Die Person gibt ihren Arbeitsplatz frei und sucht einen anderen erreichbaren, freien Platz. Bis dahin entfällt ihr Arbeitsweg. Wird ihre Wohnung zurückgegeben, zieht der Haushalt gemeinsam aus und gibt seine übrigen Plätze frei. Die Vorschau zählt ausziehende Haushalte und verbleibende Personen mit verlorenem Arbeitsplatz getrennt.
- Wird ein mittleres Geschoss eines einfachen Lifts zurückgegeben, bleibt die vom ursprünglichen unteren Endhalt aus zusammenhängende Strecke mit mindestens zwei Halten betriebsfähig. Der obere Teil wird stillgelegt. Entfällt der untere Endhalt oder bleibt nur ein Halt, endet der gesamte Betrieb. Die Vorschau markiert die verbleibenden Halte und betroffenen Fahrgäste. Eine Kabine im entfallenden Teil wird bei der Rückgabe geleert. Verbleibende Fahrgäste erreichen nach der oben beschriebenen Anschlussregel einen gültigen Ort und planen von dort neu. Ein später neu angemietetes Geschoss kann der Spieler wieder erschließen.

## Darstellung, Inhalte und Eingabe

Die Godot-Welt zeigt den frontalen Schnitt der Stilreferenz. Ein Etagenabschnitt bündelt statische Architektur und Einrichtung. Türen, Kabinen, Personen, mitgeführte Gegenstände und anklickbare Hinweise bilden getrennte Zeichenebenen. Beim Zoomen und Schwenken fordert der Client den sichtbaren Bereich mit einem kleinen Rand an. Er verwendet Figurenansichten erneut und löst stabile Personen-IDs beim Anklicken auf. In der Übersicht reichen günstigere Animationen, in der Nähe werden Becher, Blicke und Gesten sichtbar. Die Simulationsgenauigkeit bleibt bei beiden Zoomstufen gleich.

Der erste Grafiksatz umfasst ein modulares Büro, Wohnung, Rezeption, Kiosk, Treppe, Tür, Kabine sowie wenige Figuren mit getrennten Requisiten. Ein gemeinsames Pixelraster, dokumentierte Fuß- und Türanker, Sprite-Atlanten und einheitliche Importfilter halten Figuren beim Übergang zwischen Raum und Lift auf derselben Höhe. Das erste Probespiel prüft die Lesbarkeit in Übersicht und Nahansicht direkt gegen die [finale Stilreferenz](../../../konzepte/finale-stilreferenz.png).

Grundrisse und Regeln liegen als versionierte, engineunabhängige Datendateien vor. Der Loader prüft eindeutige IDs, gültige Verweise, Rastergrenzen und Halte. Eine separate Zuordnung verbindet visuelle Typen mit Godot-Ressourcen. Für Maus und Tastatur werden Aktionen wie Auswählen, Verschieben, Zoomen und Bau bestätigen definiert. Personeninformationen lassen sich durch Auswahl öffnen. Spätere Touch-Gesten können dieselben Aktionen auslösen. Schriftgrößen, Treffflächen, Speicher und Leistung auf echten Mobilgeräten bleiben eigenständige Abnahmepunkte für eine Mobilfassung.

## Spielstände und Auslieferung

Der Kern erzeugt an einer abgeschlossenen Tickgrenze einen versionierten Schnappschuss. Er enthält Szenario- und Regelversion, Spielzeit, nächsten ID-Wert, Zufallszustand, offene Termine und angenommene zukünftige Befehle sowie Bewohner, Wegefortschritt, Warteschlangen, Kabinen, Verträge und Kasse. Abgeleitete Indizes werden beim Laden neu aufgebaut. Als erstes Format schlage ich strukturiertes JSON vor, damit sich Fehlerfälle untersuchen lassen. Dateigröße und Ladezeit des großen Hauses entscheiden über spätere Kompression. Ein Ladeversuch prüft Version und Referenzen in einer separaten Kerninstanz. Erst nach Erfolg ersetzt sie die aktive Partie. Der Client schreibt zunächst eine temporäre Datei und ersetzt dann den letzten vollständigen Stand über die Speicherfunktion der jeweiligen Plattform. Die vorherige gültige Generation bleibt als Rückfall verfügbar.

Der Build fixiert die verwendete Godot-4-Version, die kompatible Rust-Anbindung und die Exportvorlagen gemeinsam. Jeder Desktop-Export enthält die passende native Bibliothek. Ein Starttest des **exportierten** Spiels auf Windows und Linux lädt das kleine Haus, nimmt einen Befehl an und lädt einen Spielstand wieder. Eine Mobilfassung benötigt passende native Builds, Touch-Bedienung und einen Test für Speichern beim Wechsel in den Hintergrund. Ein späterer Browserexport erfordert einen eigenen Nachweis für die WebAssembly-Anbindung, den verwendeten Godot-Renderer und dauerhafte Speicherung im Browser. Die Desktop-Entscheidung enthält keine Zusage über die Leistung dieser späteren Fassungen.

## Messung und Abnahme

Die Zahlen sind erste Prüfschwellen. Vor der ersten Vergleichsmessung wird ein tatsächlich verfügbares Referenzgerät mit CPU, GPU, RAM, Betriebssystem und Auflösung protokolliert. Bis dahin gelten sie als Budget. Zeitmessungen laufen in Release-Builds mit identischem Szenario und Startwert. Für den Vergleich werden Tagesanlauf, Stoßzeit und Ruhephase getrennt ausgewiesen.

| Prüfung | Konkreter Nachweis |
| --- | --- |
| Unabhängigkeit vom Client | Der Kern lässt sich in einer Umgebung ohne Godot bauen und testen. Ein kompletter Spieltag mit Einzug, Liftbau und Rückgabe läuft dort durch. Eine automatisierte Godot-Partie mit derselben Befehlsfolge erreicht an Kontrollticks dieselben Kernprüfsummen. |
| Unabhängigkeit von der Kamera | Zwei Durchläufe mit gleichem Startwert beobachten unterschiedliche Etagen. Personen, Wartezeiten, Geld und Endzustand stimmen überein. |
| Lebenszyklus eines Spielstands | Mitten im Umsteigen speichern, laden und einen festgelegten Zeitraum fortsetzen. Vergleich mit durchlaufender Partie einschließlich Reihenfolge der Wartenden und nächster Zufallsentscheidung. Abgebrochener Schreibvorgang, unbekannte Version und ungültiger Verweis werden gesondert geprüft. |
| Stau mit Ursache | Zwei identische Stoßzeiten mit schmalem und breitem Umsteigebereich vergleichen. Der breitere Bereich verkürzt die gemessene Wartezeit im dafür gebauten Testszenario. Die Ursache ist beim betroffenen Fahrgast abrufbar. Unter endlicher Nachfrage leert sich die Schlange. |
| Größenordnung | 100 Geschosse, zunächst 5.000 Personen. Mindestens 15 reale Minuten mit Stoßzeit messen. Vorschlag für Normaltempo ist eine Tickzeit von höchstens 8 ms im 95. Perzentil. Zusätzlich Maximum, Zahl der Wegsuchen, längste Warteschlange und Spitzenspeicher festhalten. |
| Sichtbarer Betrieb | Auf dem Referenz-PC bei 1080p auf 60 Bilder/s zielen. Bildzeiten einschließlich 95. und 99. Perzentil beim Schwenken über einen vollen Umstieg und beim Zoomen messen. Separat erfassen, wie viel Zeit Kern, Adapter und Darstellung beanspruchen. |
| Spielbarkeit | Der erste Arbeitsweg, der geänderte Weg nach Liftbau, die Bewohnerabkürzung, die Folgen einer Rückgabe und zwei gleichzeitig erkennbare Alltagshandlungen lassen sich im exportierten Spiel beobachten. |

## Reihenfolge und Entscheidungspunkte

1. **Lauffähige Verbindung zwischen Kern und Client.** Kleinstes Haus als Datendatei, eine gehende Person, ein Spielbefehl, zwei Aufrufer. Erfolgsnachweis sind übereinstimmende Kernzustände im Szenarioprogramm und im exportierten Godot-Spiel. Dieser Schritt klärt früh das Risiko der nativen Anbindung.
2. **Erster Arbeitstag.** Rezeption, Abschnittsmiete, Widmung, Einzug, Treppenweg und Kasse umsetzen. Eine Person vom Einzug bis zur Bürotür verfolgen. Im selben Szenario Speichern und Laden während des Gehens nachweisen.
3. **Verkehr unter Last.** Kabinenbetrieb, Halte, Platzverbrauch, Umstieg und Auskünfte ergänzen. Parallel zum kleinen Handlungsbeispiel das 100-Geschosse-Szenario als reproduzierbare Lastmessung aufbauen. Erst anhand dieser Messung über Routen-Caches, feinere räumliche Indizes oder einen Simulationsthread entscheiden.
4. **Eigeninitiative und Rückgabe.** Wiederholten Kaffeegang, beobachtbaren Bau der Rutschstange, Freischaltung und atomare Rückgabe durchspielen. Unterbrochene Reisen, entfernte Ziele und belegte Kabinen als Regressionen prüfen.
5. **Bildsprache und erster PC-Build für Probespiele.** Animationen, Figurenwahl, Übersicht, Zoom und Erklärtexte am kleinen Haus abstimmen. Beide Desktop-Exporte mit Speichern und Laden testen. Fachliche Abnahme durch Beobachten des Hauses, technische Abnahme anhand der Messungen oben.

Für die Umsetzung offen bleiben Bibliotheksversionen innerhalb der gewählten Architektur, Sprite-Maße nach einer Probe mit der Stilreferenz, Balancingwerte sowie durch Messungen begründete Optimierungen. Ein möglicher Paternoster und zusätzliche Einrichtungen folgen den späteren Entscheidungen aus dem Spielkonzept. Die erste Fassung ist als lokales Einzelspiel vorgeschlagen.
