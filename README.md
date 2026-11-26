<img src="assets/icon.svg" width="64" height="64" alt="">

# Silly Office

Ein Gebäudeschnitt-Prototyp mit einem eigenständigen Rust-Simulationskern und einem Godot-Client. Im kleinen Haus richtest du Wohnungen und Arbeitsplätze ein, beobachtest Einzüge und entscheidest, wo sich ein Lift lohnt.

## Prototyp spielen

Starte `SillyOffice` aus dem Linux- oder Windows-Archiv. Die native Bibliothek bleibt neben dem Programm. Die Partie beginnt pausiert.

1. Wähle unter **Miete & Raum** Fläche 6 im Erdgeschoss als **Wohnung** und Fläche 7 im zweiten Obergeschoss als **Büro**. Widme beide Flächen, dann setze die Zeit fort oder gehe einen Tick weiter.
2. Wähle die eingezogene Person per Klick oder aus der Liste. **Person folgen** schwenkt auf ihre Etage. Übersicht, Etagenwahl, Pause und Einzelschritt bleiben unabhängig vom Kernzustand.
3. Verwende den Entwicklungspunkt, um den einfachen Lift freizuschalten. Wähle die Endhalte H1 und H3. Gangwidmungen verbreitern den Vorraum.
4. Widme Fläche 9 zum **Kiosk**. Schicke die Person vom Büro aus mehrmals zum Kiosk. Wiederholte Umwege können eine abwärts führende Rutschstange auslösen.
5. Wähle einen gemieteten Abschnitt und **Rückgabe prüfen**. Die Vorschau zeigt Mietänderung, Auszüge, Arbeitsplatzverluste und betroffene Fahrgäste. Mit **Bestätigen** führst du die gezeigte Rückgabe am nächsten Tick aus. **Speichern** und **Laden** setzen auch eine laufende Fahrt fort.

Die [Desktop-CI](https://github.com/mpwg/sillyoffice/actions/workflows/desktop.yml) stellt die nativen Programme und Prüfberichte als Archive bereit.

## Kern und CLI

Rust und Cargo sind in [rust-toolchain.toml](rust-toolchain.toml) festgelegt. Szenarien und Abläufe sind versioniertes JSON.

```
cargo test --locked -p sim-core -p sim-cli
mkdir -p build
cargo run --locked -p sim-cli -- replay godot/content/erstes-haus.json godot/content/spieltag.json --out build/spieltag-cli.json
cargo run --locked -p sim-cli -- replay godot/content/zwei-etagen.json godot/content/erster-weg.json --out build/cli.json
```

Der [Spieltag](godot/content/spieltag.json) umfasst Einzug, Liftbau, zwei Kaffeegänge, eine Bewohnerinitiative und bestätigte Abschnittsrückgabe. Der [bisherige Arbeitsweg](godot/content/erster-weg.json) prüft den Kernzustand vor und nach der Anmietung bei Tick 4 und 5 sowie am Büro bei Tick 12.

Ein kurzer Lauf des 100-Etagen-Hauses erzeugt einen Messbericht und das vollständig aufgebaute Szenario. Für eine reale Viertelstunde verwende `--seconds 900` anstelle von `--ticks 5000`.

```
cargo run --locked --release -p sim-cli -- bench godot/content/berufsverkehr.json --ticks 5000 --out build/last-kurz.json --scenario-out build/berufsverkehr-haus.json
```

## Linux-Export bauen und prüfen

Verwende Godot **4.6 stable** mit passenden Exportvorlagen. Die Erweiterung nutzt godot-rust **0.5.6** mit API **4.6**. Die Frame-Verzögerung beim ersten Import umgeht einen Absturz bei der Dokumentationserzeugung der GDExtension. [Godot-Fehlerbericht](https://github.com/godotengine/godot/issues/111048)

```
cargo build --locked --release -p sim-godot
mkdir -p godot/bin build/prototyp-linux
cp target/release/libsim_godot.so godot/bin/
godot --headless --frame-delay 1000 --path godot --import
godot --headless --path godot --export-release Linux ../build/prototyp-linux/SillyOffice
cd build/prototyp-linux
./SillyOffice
```

Die folgenden Befehle benötigen den oben erzeugten CLI-Bericht in `build/spieltag-cli.json`. Die beiden Neustartphasen laufen als getrennte Prozesse. Phase 1 speichert bei Tick 35 in der Kabine. Phase 2 lädt diesen Stand und setzt den Spieltag bis Tick 100 fort.

```
./SillyOffice --headless -- --replay spieltag --out spieltag.json --reference ../spieltag-cli.json --camera-alternate
./SillyOffice --headless -- --test desktop-smoke --phase save --file smoke-save.json --out smoke-before.json
./SillyOffice --headless -- --test desktop-smoke --phase resume --file smoke-save.json --out smoke-after.json
```
