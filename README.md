<img src="assets/icon.svg" width="64" height="64" alt="">

# Silly Office

Im Zwei-Etagen-Haus geht eine bereits eingezogene Person von der Rezeption über die Treppe ins Büro. Bei Tick 5 mietet die Simulation einen freien Seitenflügel. Die CLI und das exportierte Spiel verwenden dieselben [Hausdaten](godot/content/zwei-etagen.json), denselben [Ablauf](godot/content/erster-weg.json) und denselben Rust-Kern.

| Tick | Erwarteter Kernzustand |
| --- | --- |
| 4 | Der Seitenflügel ist frei. Die Person ist auf der Treppe. |
| 5 | Befehl 1 mietet den Flügel für 25 Geldeinheiten. Befehl 2 wird als Doppelanmietung abgelehnt. |
| 12 | Dieselbe Person ist im Büro angekommen. |

## CLI ohne Godot

Die Version von Rust und Cargo steht in [rust-toolchain.toml](rust-toolchain.toml).

```
cargo test --locked -p sim-core -p sim-cli
mkdir -p build
cargo run --locked -p sim-cli -- replay godot/content/zwei-etagen.json godot/content/erster-weg.json --out build/cli.json
```

## Linux-Export prüfen

Verwende Godot **4.6 stable** mit den passenden Exportvorlagen. Der Build nutzt godot-rust **0.5.6** mit API **4.6**. Die Frame-Verzögerung beim ersten Import umgeht einen Absturz bei der Dokumentationserzeugung der GDExtension. [Godot-Fehlerbericht](https://github.com/godotengine/godot/issues/111048)

```
cargo build --locked --release -p sim-godot
mkdir -p godot/bin build/linux
cp target/release/libsim_godot.so godot/bin/
godot --headless --frame-delay 1000 --path godot --import
godot --headless --path godot --export-release Linux ../build/linux/SillyOffice
cd build/linux
./SillyOffice --headless -- --replay erster-weg --out godot.json --reference ../cli.json
./SillyOffice --headless -- --replay erster-weg --out godot-alternate.json --reference ../cli.json --camera-alternate
```

Beide Läufe vergleichen Tick, Befehlsergebnisse, SHA-256-Prüfsumme und die vollständigen Kernzustände. Bei einer Abweichung meldet der Export den ersten betroffenen Kontrolltick mit beiden Zuständen. Starte `SillyOffice` ohne Argumente, um den Ablauf im Fenster zu sehen. Die Schaltflächen wechseln zwischen Hausübersicht, Rezeption und Büro, auch während der Pause.

[GitHub Actions](https://github.com/mpwg/sillyoffice/actions/workflows/desktop.yml) baut und prüft die Exporte für Linux und Windows nativ. Jeder erfolgreiche Job stellt das Programm mit seiner Bibliothek und den Vergleichsberichten als Archiv bereit.
