extends RefCounted

const Replay := preload("res://tests/replay.gd")
const SAVE_TICK := 35


static func run(partie, args: PackedStringArray) -> int:
	var options := {}
	if args.size() != 6:
		return _usage()
	for index in range(0, args.size(), 2):
		var key := args[index]
		var value := args[index + 1]
		if key not in ["--phase", "--file", "--out"] or options.has(key):
			return _usage()
		if value.is_empty() or value.begins_with("--"):
			return _usage()
		options[key] = value
	var phase: String = options["--phase"]
	if phase not in ["save", "resume"]:
		return _usage()

	var save_path := _absolute_path(options["--file"])
	var output_path := _absolute_path(options["--out"])
	if output_path in [save_path, save_path + ".previous", save_path + ".tmp"]:
		return _fail("Spielstand und Kontrollpunkt brauchen eigene Dateien.")

	if not FileAccess.file_exists(Replay.PLAY_DAY_REPLAY):
		return _fail("Replay-Ablauf fehlt (%s)." % Replay.PLAY_DAY_REPLAY)
	var replay: Variant = JSON.parse_string(FileAccess.get_file_as_string(Replay.PLAY_DAY_REPLAY))
	if replay is not Dictionary or replay.get("checkpoints") is not Array:
		return _fail("Kontrollticks fehlen im Spieltag.")
	var checkpoints: Array = replay.checkpoints
	if not checkpoints.has(float(SAVE_TICK)) or int(checkpoints.back()) <= SAVE_TICK:
		return _fail("Der Spieltag enthält keine Kontrollticks für Speichern und Fortsetzen.")

	partie.set_paused(true)
	var report: String
	if phase == "save":
		var load_error: String = partie.start_partie(Replay.PLAY_DAY_SCENARIO, Replay.PLAY_DAY_REPLAY)
		if not load_error.is_empty():
			return _fail(load_error)
		report = _advance(partie, 0, SAVE_TICK)
		if report.is_empty():
			return 1
		if not _rider_present(report):
			return _fail("Bei Tick %d ist Person 1 nicht im Lift." % SAVE_TICK)
		var save_error: String = partie.save_game(save_path)
		if not save_error.is_empty():
			return _fail(save_error)
	else:
		var load_error: String = partie.load_game(save_path, Replay.PLAY_DAY_SCENARIO)
		if not load_error.is_empty():
			return _fail(load_error)
		if partie.render_tick() != SAVE_TICK or not _rider_present(partie.checkpoint_json()):
			return _fail("Der geladene Spielstand zeigt Person 1 bei Tick %d nicht im Lift." % SAVE_TICK)
		report = _advance(partie, SAVE_TICK, int(checkpoints.back()))
		if report.is_empty():
			return 1

	var output := FileAccess.open(output_path, FileAccess.WRITE)
	if output == null:
		return _fail("Kontrollpunkt kann nicht geöffnet werden (%s)." % error_string(FileAccess.get_open_error()))
	if not output.store_line(report):
		return _fail("Kontrollpunkt kann nicht geschrieben werden (%s)." % error_string(output.get_error()))
	return 0


static func _advance(partie, from_tick: int, to_tick: int) -> String:
	var report := ""
	for tick in range(from_tick + 1, to_tick + 1):
		if not partie.advance_tick():
			_fail("Der Spieltag blieb bei Tick %d stehen." % (tick - 1))
			return ""
		report = partie.checkpoint_json()
		var checkpoint: Variant = JSON.parse_string(report)
		if checkpoint is not Dictionary or checkpoint.get("tick", -1) != tick:
			_fail("Ungültiger Kontrollpunkt für Tick %d." % tick)
			return ""
		for result in checkpoint.results:
			if result.outcome.kind != "applied":
				_fail(
					"Pflichtbefehl %d bei Tick %d abgelehnt (%s)."
					% [result.number, tick, JSON.stringify(result.outcome)]
				)
				return ""
	return report


static func _rider_present(report: String) -> bool:
	var checkpoint: Dictionary = JSON.parse_string(report)
	var person: Dictionary = checkpoint.state.people.get("1", {})
	return person.get("location", {}).get("kind") == "riding_lift"


static func _absolute_path(path: String) -> String:
	var expanded := ProjectSettings.globalize_path(path)
	if expanded.is_relative_path():
		expanded = DirAccess.open(".").get_current_dir().path_join(expanded)
	return expanded.simplify_path()


static func _usage() -> int:
	return _fail("Verwende --test desktop-smoke --phase save|resume --file SPIELSTAND --out REPORT.")


static func _fail(message: String) -> int:
	printerr(message)
	return 1
