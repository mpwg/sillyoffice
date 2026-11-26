# Kernmessung des Berufsverkehrs

Gemessen am 26. November 2026 auf einem AMD Ryzen 5 5600 mit 6 Kernen und 12 Threads, 31,3 GiB RAM und CachyOS Linux 7.3.6-1-cachyos (x86_64). Der Release-Build stammt aus Commit `44ec38b` mit Rust 1.100.0. Die Messung erfasst `World::step()` ohne Renderer. Für die grafische Probe bei 1.920 × 1.080 lief der separate [Desktop-Test](probespiel.md). Kurze Build- und Grafikproben überlappten mit dem Anlauf.

Das [Lastszenario](../../godot/content/berufsverkehr.json) erzeugt 100 Etagen und 5.000 Personen mit Saat 42. Die Nachfragewelle startet in Tick 4.600. Pro Etage haben fünf Personen ein Ziel auf einer anderen Etage. Der Express hält auf den Etagen 0, 10 bis 90.

| Phase | Ticks | p95 in ms | Maximum in ms | Wegsuchen | Wartende nach einem Tick, max. | RSS-Spitze in MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Anlauf | 4.599 | 0,088 | 0,787 | 0 | 0 | 8,45 |
| Stoßzeit | 3.300 | 0,220 | 9,194 | 388 | 499 | 8,45 |
| Ruhephase | 1.101 | 0,089 | 0,140 | 0 | 0 | 8,45 |

Alle 9.000 Ticks liefen im Normaltempo von zehn Ticks pro Sekunde. Die Laufzeit betrug 900,000 Sekunden und schließt die Wartezeit zwischen den Ticks ein. Der p95 lag in allen drei Phasen unter dem Budget von 8 ms. Das Prozesshoch lag bei 8,50 MiB RSS.

Während eines Ticks warteten bis zu 500 Personen. Insgesamt gab es 2.555 Einstiege und 1.640 Umstiege. Person 1 wechselte in Tick 4.669 von Expresslift 11 zu Zubringer 3. Am Ende wartete niemand. Der aktuelle Build lieferte in einem ungedrosselten Lauf über dieselben 9.000 Ticks identische fachliche Kennzahlen und dieselbe Prüfsumme:

```text
80e8ea57afbba65b769eb69069a0b3c0132bbefd26dbe3401bc07623754b4dfa
```

Der Rohbericht liegt lokal unter `build/last.json`. Die Hausdatei und der Bericht lassen sich mit diesem Lauf reproduzieren:

```
cargo run --locked --release -p sim-cli -- bench godot/content/berufsverkehr.json --seconds 900 --out build/last.json --scenario-out build/berufsverkehr-haus.json
```
