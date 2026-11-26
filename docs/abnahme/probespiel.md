# Abnahme des Desktop-Prototyps

Der Prototyp verwendet den vereinfachten Gebäudeschnitt des Drei-Etagen-Hauses. Die Bedienung und die Bauvorschau laufen über dieselben Kernbefehle wie in der CLI. Der exportierte Client überschreibt beim Laden die aktive Partie erst nach erfolgreicher Validierung.

| Kriterium | Prüfnachweis |
| --- | --- |
| Einzug und Kasse | `first_day` prüft erreichbare Arbeit, dieselbe Personen-ID bis ins Büro und die Tagesabrechnung. |
| Liftbau und Vorraum | `lift` und `interchange` prüfen 15 zusammenhängende Halte, Sperren, konkurrierende Bauaufträge, den Wechsel von der Treppe zum Lift und kürzere Wartezeit für dieselbe Gruppe nach der Verbreiterung. Die Lifttests prüfen Ausstiegsvorrang, Richtungswechsel und die Reihenfolge bei gleichem Ankunftstick. |
| Fortsetzung | `save_resume` prüft den offenen Weg, ausstehende Befehle, den nächsten Zufallsentscheid und die Reihenfolge in einer besetzten Kabine mit Wartenden. Der Clienttest `save-store` prüft unterbrochene Schreib- und Austauschschritte. |
| Kaffeegang und Rückgabe | `coffee` prüft Entstehung, Richtung und Wiederaufnahme der Rutschstange. `return_section` prüft erhaltene Wohnungen, gemeinsame Auszüge, eine veraltete Bestätigung ohne Zustandsänderung und wiederherstellbare Spielstände nach einer Rückgabe während des Gehens oder der Liftfahrt. |
| Bedienoberfläche | `selection` prüft die Auswahl über einen Klick und dieselbe Personen-ID beim Etagenwechsel. Die Rückgabevorschau nennt Mietänderung, Auszüge, Arbeitsplatzverluste, betroffene Personen und verbleibende Halte. |
| Gleichlauf der Desktop-Exporte | Die nativen Linux- und Windows-Jobs vergleichen den vollständigen JSON-Zustand an allen 13 Kontrollticks von `spieltag` mit der CLI. Ein exportierter Prozess speichert bei Tick 35 in der Kabine. Ein neuer Prozess lädt bis Tick 100 und liefert die gleiche Prüfsumme und den gleichen Zustand wie der durchgehende Lauf. |

## Grafische Probe

Der Linux-Release lief bei 1.920 × 1.080 in Gamescope/Xwayland mit OpenGL 3.3 auf einer GeForce RTX 3060 Ti. Wohnung, Büro und Kiosk wurden per Maus gewidmet. Die Figur ließ sich im Gebäudeschnitt auswählen und über den Etagenwechsel verfolgen. In Tick 46 wurde während der Kaffeefahrt im Lift gespeichert. Ein frischer Prozess stellte die Fahrt wieder her. In Tick 58 war dieselbe Person auf dem Rückweg zum Büro. Nach Rückgabe des Abschnitts im ersten Obergeschoss zeigte die Bauvorschau H2 als ersten blockierten Halt.

Godot 4.6 meldet beim Öffnen und Schließen mancher Auswahllisten Fokusfehler im Log. Die Auswahllisten blieben in der grafischen Probe bedienbar. [Bestätigte Godot-Regression](https://github.com/godotengine/godot/issues/89657)

## Downloads

Der native Exportlauf für [1126fed](https://github.com/mpwg/sillyoffice/actions/runs/45784188235) enthält einen Download je Plattform mit dem Programm, der Erweiterung und den Testberichten. Die lokale Start- und Bauanleitung steht in der [README](../../README.md).
